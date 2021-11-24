use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::schemas::money::Price;

#[derive(Deserialize, Serialize, Hash, PartialEq, Eq)]
pub enum CPUBenchmarkMetric {
    #[serde(rename = "passmark")]
    Passmark,
}

#[derive(Deserialize, Serialize, Default, Hash, PartialEq, Eq)]
pub struct CPUBenchmark {
    pub overall: u32,
    pub thread: Option<u32>,
}

#[derive(Deserialize, Serialize, Default)]
pub struct CPU {
    pub passmark_id: Option<u32>,
    pub name: String,
    pub benchmarks: HashMap<CPUBenchmarkMetric, CPUBenchmark>,
    pub socket: Option<String>,
    pub sector: Option<String>,
    pub cores: Option<u32>,
    pub logicals: Option<u32>,
    pub price: Option<Price>,
    pub tdp: Option<u32>,
}
