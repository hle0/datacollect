use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DefaultOnError, DisplayFromStr, PickFirst};

use crate::common::{Client, IgnoreComma, Money};

#[serde_as]
#[derive(Deserialize, Serialize)]
pub struct CPU {
    #[serde_as(as = "PickFirst<(_, DisplayFromStr)>")]
    pub id: u32,
    pub name: String,
    #[serde(default)]
    #[serde_as(as = "DefaultOnError<PickFirst<(_, Option<IgnoreComma<Money>>)>>")]
    pub price: Option<Money>,
    #[serde(default)]
    #[serde_as(as = "DefaultOnError<PickFirst<(_, Option<IgnoreComma<u32>>)>>")]
    pub cpumark: Option<u32>,
    #[serde(default)]
    #[serde_as(as = "DefaultOnError<PickFirst<(_, Option<IgnoreComma<u32>>)>>")]
    pub thread: Option<u32>,
    pub socket: String,
    pub cat: String,
    #[serde(default)]
    #[serde_as(as = "DefaultOnError<PickFirst<(_, Option<DisplayFromStr>)>>")]
    pub cores: Option<u32>,
    #[serde(default)]
    #[serde_as(as = "DefaultOnError<PickFirst<(_, Option<DisplayFromStr>)>>")]
    pub logicals: Option<u32>,
    #[serde_as(as = "DefaultOnError<PickFirst<(_, Option<DisplayFromStr>)>>")]
    pub tdp: Option<f64>,
}

#[derive(Serialize, Deserialize)]
pub struct CPUMegaList {
    data: Vec<CPU>,
}

impl CPUMegaList {
    /// Get the big list of CPU's from Passmark's website.
    ///
    /// # Errors
    /// Errors if one of the requests failed, or if parsing one of the responses failed.
    pub async fn get(client: &mut Client<true>) -> anyhow::Result<Self> {
        /* there's a session cookie we need here */
        client
            .0
            .get("https://www.cpubenchmark.net/CPU_mega_page.html")
            .send()
            .await?;

        let res = client
            .0
            .get("https://www.cpubenchmark.net/data/")
            .header("X-Requested-With", "XMLHttpRequest")
            .send()
            .await?;

        let json: Self = res.json().await?;
        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use crate::common::Client;

    use super::CPUMegaList;

    #[tokio::test]
    async fn test_producer() {
        let mut client = Client::<true>::default();
        let cpus = CPUMegaList::get(&mut client).await.unwrap();
        let my_cpu = cpus
            .data
            .iter()
            .find(|cpu| cpu.name == "AMD Ryzen 5 2600")
            .unwrap();
        assert_eq!(my_cpu.tdp, Some(65.0));
    }
}
