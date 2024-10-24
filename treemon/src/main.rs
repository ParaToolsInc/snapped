mod protocol;
mod tbon;
mod utils;
mod webserver;

use std::{
    env,
    process::{Command, Stdio},
    time::Instant,
};

use anyhow::Result;
use clap::Parser;

use crate::tbon::Tbon;

#[derive(Parser)]
struct Arg {
    /// Number of processes to join
    #[clap(short = 'n', long)]
    number: Option<usize>,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    command: Option<Vec<String>>,
}

fn timer_print(text: &str, start: Instant) {
    println!(
        "{} in {} seconds",
        text,
        start.elapsed().as_millis() as f64 / 1000.0
    );
}

fn be_root_server(child_count: usize, cmd: &Option<Vec<String>>) -> Result<()> {
    let tbon = Tbon::init_as_root(None)?;

    if let Some(command) = cmd {
        env::set_var("TREEMON_ROOT", tbon.url());

        let _ = Command::new(&command[0])
            .args(&command[1..])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .spawn()?;
    }

    let bstart = Instant::now();

    tbon.wait_for_child(child_count)?;

    timer_print(
        &format!("Built a tree of {} processes", child_count),
        bstart,
    );

    println!("All processes joined root server");

    webserver::serve_clients(tbon)?;

    Ok(())
}

fn be_leaf_server() -> Result<()> {
    let tbon = Tbon::init_as_leaf()?;

    let mut cnt = 0;

    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));

        tbon.set_counter("test", cnt);

        cnt = (cnt + 1) % 10;
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Arg::parse();

    /* If env is set we are leaf */
    if std::env::var("TREEMON_ROOT").is_ok() {
        /* Done */
        be_leaf_server()?;
        return Ok(());
    }

    /* If we are here we are the root */

    let number = args.number.unwrap_or(1);
    be_root_server(number, &args.command)?;

    Ok(())
}
