use std::collections::{HashMap, HashSet};

use histogram::Histogram;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ClientInfo {
    pub id: TbonId,
    pub cmd: ClientCommand,
}

pub type TbonId = u32;

#[derive(Serialize, Deserialize)]
pub enum ClientCommand {}

#[derive(Serialize, Deserialize)]
pub enum TbonQuery {
    Join(TbonId),
    Pivot(String),
    Count,
    Histogram(String),
    ListKeys,
    Values(String),
}

#[derive(Serialize, Deserialize)]
pub enum TbonResponse {
    Err(String),
    Ok,
    Count(u32),
    Pivot(TbonId, String),
    Histogram(f64, Histogram),
    ListKeys(HashSet<String>),
    Values(HashMap<String, u64>),
}
