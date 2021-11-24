use async_trait::async_trait;
use maplit::hashmap;
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::{
    common::{parse_dollars, DataProducer, Depth},
    schemas::{
        computing::{CPUBenchmark, CPUBenchmarkMetric, CPU},
        money::{Currency, Price},
    },
};
use reqwest::Client;
use std::convert::TryInto;

pub struct PassmarkCPUDataSource {
    client: Client,
    initialized: Mutex<bool>,
}

impl PassmarkCPUDataSource {
    /// Create a new instance.
    ///
    /// # Errors
    /// Errors if the [`reqwest::Client`] couldn't be built.
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            client: Client::builder().cookie_store(true).build()?,
            initialized: Mutex::new(false),
        })
    }
}

#[derive(Deserialize)]
struct RawCPUBenchmark {
    pub id: String,
    pub name: String,
    pub price: String,
    pub cpumark: String,
    pub thread: String,
    pub socket: String,
    pub cat: String,
    pub cores: String,
    pub logicals: String,
    pub tdp: String,
}

impl std::convert::TryInto<CPU> for RawCPUBenchmark {
    type Error = anyhow::Error;
    fn try_into(self) -> anyhow::Result<CPU> {
        Ok(CPU {
            passmark_id: Some(self.id.parse()?),
            benchmarks: {
                let benchmarks: anyhow::Result<_> = try {
                    hashmap! {
                        CPUBenchmarkMetric::Passmark => CPUBenchmark {
                            overall: self.cpumark.replace(",", "").parse()?,
                            thread: self.thread.replace(",", "").parse().ok(),
                        }
                    }
                };

                benchmarks
                    .map_err(|_| std::collections::HashMap::default())
                    .into_ok_or_err()
            },
            name: self.name,
            socket: Some(self.socket),
            sector: Some(self.cat),
            cores: self.cores.replace(",", "").parse().ok(),
            logicals: self.logicals.replace(",", "").parse().ok(),
            price: try {
                Price {
                    unit: Currency::USD,
                    amount: f64::from(parse_dollars(self.price)?) / 100.0,
                }
            },
            tdp: self.tdp.replace(",", "").parse().ok(),
        })
    }
}

#[derive(Deserialize)]
pub struct RawCPUBenchmarkJSONContainer {
    data: Vec<RawCPUBenchmark>,
}

#[async_trait]
impl DataProducer<Vec<CPU>> for PassmarkCPUDataSource {
    async fn produce(&mut self, _depth: Depth) -> anyhow::Result<Vec<CPU>> {
        {
            let mut inited = self.initialized.lock().await;
            if !*inited {
                /* there's a session cookie we need here */
                self.client
                    .get("https://www.cpubenchmark.net/CPU_mega_page.html")
                    .send()
                    .await?;
                *inited = true;
            }
        }

        let res = self
            .client
            .get("https://www.cpubenchmark.net/data/")
            .header("X-Requested-With", "XMLHttpRequest")
            .send()
            .await?;
        let json: RawCPUBenchmarkJSONContainer = res.json().await?;
        let data: Vec<CPU> = json
            .data
            .into_iter()
            .map(RawCPUBenchmark::try_into)
            .filter(Result::is_ok)
            .map(Result::unwrap)
            .collect::<Vec<CPU>>();
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::DataProducer;

    use super::PassmarkCPUDataSource;

    #[tokio::test]
    async fn test_producer() {
        let mut src = PassmarkCPUDataSource::new().unwrap();
        let cpus = src.produce(crate::common::Depth::Default).await.unwrap();
        let my_cpu = cpus
            .iter()
            .find(|cpu| cpu.name == "AMD Ryzen 5 2600")
            .unwrap();
        assert_eq!(my_cpu.tdp, Some(65));
    }
}
