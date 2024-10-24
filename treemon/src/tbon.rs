use anyhow::anyhow;
use anyhow::Result;
use histogram::Histogram;
use rayon::prelude::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::net::TcpListener;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;

use serde_binary::binary_stream::Endian::Little;

use std::thread::spawn;

use crate::protocol::TbonId;
use crate::protocol::TbonQuery;
use crate::protocol::TbonResponse;
use crate::utils::read_command_from_sock;
use crate::utils::write_command_to_sock;

// See https://docs.rs/histogram/0.11.0/histogram/struct.Config.html#how-to-choose-parameters-for-your-data
const HIST_GROUPING: u8 = 8;
const HIST_MAX_VALUE_POW: u8 = 64;

struct Child {
    #[allow(unused)]
    id: TbonId,
    sock: TcpStream,
}

impl Child {
    fn new(id: TbonId, sock: TcpStream) -> Child {
        Child { id, sock }
    }
}

struct TreeView {
    current_id: u32,
    child: Vec<(u32, String, u32)>,
}

pub struct Tbon {
    id: TbonId,
    child: Arc<Mutex<Vec<Child>>>,
    view: Arc<Mutex<TreeView>>,
    bind_addr: String,
    counters: Arc<Mutex<HashMap<String, u64>>>,
}

impl Tbon {
    pub fn url(&self) -> String {
        self.bind_addr.clone()
    }

    fn read_response(sock: &mut TcpStream) -> Result<TbonResponse> {
        let data = read_command_from_sock(sock)?;
        let resp: TbonResponse = serde_binary::from_slice(&data, Little)?;
        Ok(resp)
    }

    fn send_response(sock: &mut TcpStream, resp: TbonResponse) -> Result<()> {
        let data = serde_binary::to_vec(&resp, Little)?;
        write_command_to_sock(sock, &data)?;
        Ok(())
    }

    fn read_query(sock: &mut TcpStream) -> Result<TbonQuery> {
        let data = read_command_from_sock(sock)?;
        let query: TbonQuery = serde_binary::from_slice(&data, Little)?;
        Ok(query)
    }

    fn send_query(sock: &mut TcpStream, info: &TbonQuery) -> Result<()> {
        let data = serde_binary::to_vec(&info, Little)?;
        write_command_to_sock(sock, &data)?;
        Ok(())
    }

    fn do_query(sock: &mut TcpStream, query: &TbonQuery) -> Result<TbonResponse> {
        Tbon::send_query(sock, query)?;
        match Tbon::read_response(sock)? {
            TbonResponse::Err(e) => Err(anyhow!(e)),
            other => Ok(other),
        }
    }

    fn do_pivot(view: Arc<Mutex<TreeView>>, new_addr: String) -> Result<(TbonId, String)> {
        if let Ok(tv) = view.lock().as_mut() {
            tv.current_id += 1;
            let new_id = tv.current_id;

            let mut target: Option<String> = None;

            for (_, url, current_count) in tv.child.iter_mut() {
                if *current_count < 16 {
                    /* Join this ID */
                    *current_count += 1;
                    target = Some(url.clone());
                    break;
                }
            }

            if let Some(targ) = target {
                /* Insert new child */
                tv.child.push((new_id, new_addr, 0));

                Ok((new_id, targ))
            } else {
                Err(anyhow!("Failed to join a root"))
            }
        } else {
            Err(anyhow!("Failed to lock children"))
        }
    }

    fn server_loop(
        child_list: Arc<Mutex<Vec<Child>>>,
        view: Arc<Mutex<TreeView>>,
        socket: TcpListener,
    ) -> Result<()> {
        loop {
            let (mut client, _) = socket.accept()?;

            /* Receive connect info */
            let query = Tbon::read_query(&mut client)?;

            let (resp, dodrop) = match query {
                TbonQuery::Join(id) => {
                    /* Push in Child list */
                    if let Ok(m) = child_list.lock().as_mut() {
                        m.push(Child::new(id, client.try_clone()?));
                    }

                    /* Note that the read thread is on the other side of the socket
                    we have commands going down (from root to client threads and down up to leaves and then up) */

                    (TbonResponse::Ok, false)
                }
                TbonQuery::Pivot(client_url) => {
                    // TODO pivot
                    let (id, url) = Tbon::do_pivot(view.clone(), client_url)?;
                    (TbonResponse::Pivot(id, url), true)
                }
                _ => (
                    TbonResponse::Err("Expected only Join or Pivot as first command".to_string()),
                    true,
                ),
            };

            Tbon::send_response(&mut client, resp)?;

            if dodrop {
                drop(client);
            }
        }
    }

    pub fn init_as_root(bind_addr: Option<&str>) -> Result<Tbon> {
        let bind: SocketAddr = bind_addr.unwrap_or("0.0.0.0:0").parse()?;

        let listening_sock = TcpListener::bind(bind)?;

        let bind_addr = format!(
            "{}:{}",
            gethostname::gethostname().as_os_str().to_str().unwrap(),
            listening_sock.local_addr().unwrap().port()
        );

        let ret = Tbon {
            id: 1,
            child: Arc::new(Mutex::new(Vec::new())),
            view: Arc::new(Mutex::new(TreeView {
                current_id: 1,
                child: vec![(1, bind_addr.clone(), 0)],
            })),
            bind_addr,
            counters: Arc::new(Mutex::new(HashMap::new())),
        };

        // Move the listening socket to a dedicated thread to handle clients
        let child_ref = ret.child.clone();
        let view_ref = ret.view.clone();

        spawn(move || {
            Tbon::server_loop(child_ref, view_ref, listening_sock).unwrap();
        });

        Ok(ret)
    }

    fn run_on_children(
        q: TbonQuery,
        child_list: Arc<Mutex<Vec<Child>>>,
    ) -> Result<Vec<TbonResponse>> {
        let resp: Vec<TbonResponse> = if let Ok(child) = child_list.lock().as_mut() {
            let resps: Vec<Result<TbonResponse>> = child
                .par_iter_mut()
                .map(|c| Tbon::do_query(&mut c.sock, &q))
                .collect();

            let mut flat_resp: Vec<TbonResponse> = Vec::new();

            for r in resps {
                match r {
                    Ok(tr) => flat_resp.push(tr),
                    Err(e) => {
                        return Err(anyhow!("Failed to run query : {}", e));
                    }
                }
            }

            flat_resp
        } else {
            return Err(anyhow!("Failed to lock child list mutex"));
        };

        Ok(resp)
    }

    fn do_count(child_list: Arc<Mutex<Vec<Child>>>) -> Result<TbonResponse> {
        /* Here we simply count our child */
        let resps = Tbon::run_on_children(TbonQuery::Count, child_list.clone())?;

        let mut total = 1;

        for r in resps {
            if let TbonResponse::Count(c) = r {
                total += c
            }
        }

        Ok(TbonResponse::Count(total))
    }

    fn do_histogram(
        key: &str,
        child_list: Arc<Mutex<Vec<Child>>>,
        counters: Arc<Mutex<HashMap<String, u64>>>,
    ) -> Result<TbonResponse> {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros();

        let mut total_h = Histogram::new(HIST_GROUPING, HIST_MAX_VALUE_POW)?;

        if let Ok(cnts) = counters.lock() {
            if let Some(c) = cnts.get(&key.to_string()) {
                total_h.increment(*c)?;
            }
        }

        let resps =
            Tbon::run_on_children(TbonQuery::Histogram(key.to_string()), child_list.clone())?;

        let mut sum_ts: f64 = ts as f64;

        for r in resps.iter() {
            if let TbonResponse::Histogram(ts, h) = r {
                sum_ts += ts;
                total_h = total_h.wrapping_add(h)?;
            }
        }

        Ok(TbonResponse::Histogram(
            sum_ts / (resps.len() + 1) as f64,
            total_h,
        ))
    }

    fn do_list_keys(
        child_list: Arc<Mutex<Vec<Child>>>,
        counters: Arc<Mutex<HashMap<String, u64>>>,
    ) -> Result<TbonResponse> {
        let resps = Tbon::run_on_children(TbonQuery::ListKeys, child_list.clone())?;

        let mut local_keys: HashSet<String> = counters.lock().unwrap().keys().cloned().collect();

        for r in resps {
            if let TbonResponse::ListKeys(k) = r {
                local_keys.extend(k);
            }
        }

        Ok(TbonResponse::ListKeys(local_keys))
    }

    fn do_values(
        key: &str,
        child_list: Arc<Mutex<Vec<Child>>>,
        counters: Arc<Mutex<HashMap<String, u64>>>,
    ) -> Result<TbonResponse> {
        let mut ret: HashMap<String, u64> = HashMap::new();

        let resps = Tbon::run_on_children(TbonQuery::Values(key.to_string()), child_list.clone())?;

        for r in resps {
            if let TbonResponse::Values(v) = r {
                ret.extend(v);
            }
        }

        let host = gethostname::gethostname()
            .as_os_str()
            .to_str()
            .unwrap()
            .to_string();
        let pid = std::process::id() as u64;

        let hostkey = format!("{}:{}", host, pid);

        if let Some(k) = counters.lock().unwrap().get(key) {
            ret.insert(hostkey, *k);
        }

        Ok(TbonResponse::Values(ret))
    }

    fn handle_query(
        q: TbonQuery,
        child_list: Arc<Mutex<Vec<Child>>>,
        counters: Arc<Mutex<HashMap<String, u64>>>,
    ) -> Result<TbonResponse> {
        let resp = match q {
            TbonQuery::Join(_) => {
                TbonResponse::Err("Only server thread should receive Join commands".to_string())
            }
            TbonQuery::Pivot(_) => {
                TbonResponse::Err("Only server thread should receive Pivot commands".to_string())
            }
            TbonQuery::Count => Tbon::do_count(child_list)?,
            TbonQuery::Histogram(key) => Tbon::do_histogram(&key, child_list, counters)?,
            TbonQuery::ListKeys => Tbon::do_list_keys(child_list, counters)?,
            TbonQuery::Values(v) => Tbon::do_values(&v, child_list, counters)?,
        };

        Ok(resp)
    }

    fn client_loop(
        sock: &mut TcpStream,
        child_list: Arc<Mutex<Vec<Child>>>,
        counters: Arc<Mutex<HashMap<String, u64>>>,
    ) -> Result<()> {
        loop {
            let query = Tbon::read_query(sock)?;

            match Tbon::handle_query(query, child_list.clone(), counters.clone()) {
                Ok(v) => Tbon::send_response(sock, v)?,
                Err(v) => Tbon::send_response(sock, TbonResponse::Err(v.to_string()))?,
            }
        }
    }

    pub fn init_as_leaf() -> Result<Tbon> {
        let mut ret = Tbon::init_as_root(None)?;

        let root_addr = std::env::var("TREEMON_ROOT")?;
        //println!("Pivoting on {}", root_addr);

        /* First pivot */
        let mut client_sock = TcpStream::connect(&root_addr)?;

        let (id, new_addr) = if let TbonResponse::Pivot(id, addr) =
            Tbon::do_query(&mut client_sock, &TbonQuery::Pivot(ret.url()))?
        {
            /* Here we should have a pivot*/
            (id, addr)
        } else {
            return Err(anyhow!("Unexpected response from Pivot"));
        };

        drop(client_sock);

        /* Set own instance ID */
        ret.id = id;

        /* Then join */
        //println!("Joining {}", new_addr);
        let mut client_sock: TcpStream = TcpStream::connect(&new_addr)?;

        /* Now we are registered in server */
        Tbon::do_query(&mut client_sock, &TbonQuery::Join(ret.id))?;

        /* Spawn client thread */
        let child_list = ret.child.clone();
        let counters = ret.counters.clone();

        spawn(move || {
            let _ = Tbon::client_loop(&mut client_sock, child_list, counters);
        });

        Ok(ret)
    }

    fn count(&self) -> Result<usize> {
        if let TbonResponse::Count(c) = Tbon::do_count(self.child.clone())? {
            return Ok(c as usize);
        }

        Err(anyhow!("Bad response type in count"))
    }

    pub fn wait_for_child(&self, count: usize) -> Result<()> {
        while self.count()? != (count + 1) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        Ok(())
    }

    pub fn set_counter(&self, counter: &str, val: u64) {
        if let Ok(cnt) = self.counters.lock().as_mut() {
            cnt.insert(counter.to_string(), val);
        }
    }

    /// Returns the histogram for a given key.
    ///
    /// This method will aggregate histograms from all children and
    /// return a single, combined histogram.
    ///
    pub fn histogram(&self, counter: &str) -> Result<(f64, Histogram)> {
        if let TbonResponse::Histogram(ts, h) =
            Tbon::do_histogram(counter, self.child.clone(), self.counters.clone())?
        {
            return Ok((ts, h));
        }

        Err(anyhow!("Bad response type in histogram"))
    }

    pub fn list_keys(&self) -> Result<HashSet<String>> {
        if let TbonResponse::ListKeys(k) =
            Tbon::do_list_keys(self.child.clone(), self.counters.clone())?
        {
            return Ok(k);
        }

        Err(anyhow!("Bad response type in listkeys"))
    }

    /// Get all values from the TBON
    /// This function is used to retrieve values for a given key.
    /// It returns a Result containing a HashMap of hostnames:pid and their corresponding values.
    pub fn values(&self, key: &str) -> Result<HashMap<String, u64>> {
        if let TbonResponse::Values(k) =
            Tbon::do_values(key, self.child.clone(), self.counters.clone())?
        {
            return Ok(k);
        }

        Err(anyhow!("Bad response type in values"))
    }
}
