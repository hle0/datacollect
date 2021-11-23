use serde::{Deserialize, Serialize};

use crate::schemas::money::Price;

#[derive(Deserialize, Serialize)]
pub enum CPUBenchmark {
    Passmark {
        total: Option<u32>,
        thread: Option<u32>,
    },
}

#[derive(Deserialize, Serialize)]
pub struct CPU {
    pub passmark_id: Option<u32>,
    pub name: String,
    pub benchmarks: Vec<CPUBenchmark>,
    pub socket: Option<String>,
    pub sector: Option<String>,
    pub cores: Option<u32>,
    pub logicals: Option<u32>,
    pub price: Option<Price>,
    pub tdp: Option<u32>,
}
