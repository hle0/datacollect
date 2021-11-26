use std::{convert::TryInto, sync::Arc, time::Duration};

use anyhow::{bail, Context};
use futures::{Stream, StreamExt};
use kuchiki::{parse_html, traits::TendrilSink};
use lazy_static::lazy_static;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::{
    common::{has_hidden_word, Client, Money},
    schema_org::Scope,
};

#[derive(Serialize)]
pub struct Seller {
    pub name: String,
    pub feedback: Option<f64>,
}

#[derive(Serialize, Default)]
pub struct Product {
    pub name: String,
    pub seller: Option<Seller>,
    pub price: Option<Money>,
    pub sponsored: Option<bool>,
}

impl Product {
    /// Find an eBay product using it's item ID.
    ///
    /// # Errors
    /// Errors if one of the requests failed, or if one of the responses could not be parsed.
    pub async fn by_id(client: &mut Client<false>, id: u64) -> anyhow::Result<Self> {
        lazy_static! {
            static ref RE_USR: regex::Regex =
                regex::Regex::new(r"https://(?:www\.)?ebay\.com/usr/([a-zA-Z0-9_\-]+)(?:\?.*)?")
                    .unwrap();
            static ref RE_PERCENT: regex::Regex =
                regex::Regex::new(r"([0-9]+(?:\.[0-9]+)?)%").unwrap();
        };

        let link = format!("https://www.ebay.com/itm/foo/{}", id);

        let response = client.0.get(link.clone()).send().await?;
        let text = response.text().await?;
        let document = kuchiki::parse_html().one(text);

        let product = try {
            let name = {
                document
                    .select_first("#itemTitle")
                    .ok()
                    .context("trying to get title")?
                    .as_node()
                    .children()
                    .find_map(|node| {
                        let s = node.as_text()?.borrow();
                        let s = s.trim();
                        if s.is_empty() {
                            None
                        } else {
                            Some(s.to_string())
                        }
                    })
                    .context("trying to get title")?
            };

            let seller: Option<Seller> = try {
                let seller_info = document.select_first(".si-content").ok()?;
                let name: String = seller_info
                    .as_node()
                    .select("a[href]")
                    .ok()?
                    .find_map(|a| {
                        let href = {
                            let attributes = a.attributes.borrow();
                            attributes.get("href")?.to_string()
                        };
                        let username = RE_USR.captures(href.as_str())?.get(1)?.as_str().to_string();
                        Some(username)
                    })?;
                let feedback: Option<f64> = try {
                    /* TODO: work on sold eBay listings (e.g. 255166134948) */
                    let text = seller_info
                        .as_node()
                        .select_first("#si-fb")
                        .ok()?
                        .as_node()
                        .text_contents();
                    let percent = RE_PERCENT.captures(text.as_str())?.get(1)?.as_str();
                    percent.parse::<f64>().ok()? * 0.01
                };

                Seller { name, feedback }
            };

            let price: Option<Money> = try {
                /* TODO: work on sold eBay listings (e.g. 255166134948) */
                let main_price = document
                    .select_first(".mainPrice")
                    .or_else(|_| document.select_first(".vi-price"))
                    .ok()?;

                let scope = Scope::from(main_price.as_node().clone());
                scope.try_into().ok()?
            };

            Self {
                name,
                seller,
                price,
                ..Default::default()
            }
        };

        product
    }

    pub fn search(query: &str) -> impl Stream<Item = anyhow::Result<Self>> + '_ {
        lazy_static! {
            static ref RE_ITM: regex::Regex =
                regex::Regex::new(r"https://(?:www\.)?ebay\.com/itm/([a-zA-Z0-9_\-]+)(?:\?.*)?")
                    .unwrap();
        }

        let stream_stream = futures::stream::iter(1..).then(move |page| {
            let ok = Arc::new(Mutex::new(true));
            let query = query.to_string();
            let client = Arc::new(Mutex::new(Client::default()));
            async move {
                {
                    let guard = ok.lock().await;
                    if !*guard {
                        bail!("something failed; pages ended, maybe?");
                    }
                }

                let text = {
                    let mut guard = client.lock().await;
                    let reqwest_client = &mut guard.0;
                    reqwest_client
                        .get("https://www.ebay.com/sch/i.html")
                        .query(&[("_nkw", query), ("_pgn", page.to_string())])
                        .send()
                        .await?
                        .text()
                        .await?
                };

                let ids = {
                    let node = parse_html().one(text);
                    let main = node
                        .select_first("#mainContent")
                        .ok()
                        .context("could not find main content")?;
                    main.as_node()
                        .select(".s-item")
                        .ok()
                        .context("could not find any items")?
                        .filter_map(|n| {
                            n.as_node()
                                .descendants()
                                .find_map(|d| {
                                    let s = d.as_element()?.attributes.borrow();
                                    let a = s.get("href")?;
                                    RE_ITM.captures(a)?.get(1)?.as_str().parse::<u64>().ok()
                                })
                                .and_then(|id| {
                                    let sponsored =
                                        n.as_node().select(".s-item__detail").ok()?.any(|e| {
                                            has_hidden_word("Sponsored", e.text_contents().as_str())
                                        });
                                    Some((id, sponsored))
                                })
                        })
                        .collect::<Vec<(u64, bool)>>()
                    /* ^ we have to collect this here because kuchiki is not thread-safe ^ */
                };

                /* make sure at least one exists */
                {
                    let mut guard = ok.lock().await;
                    *guard = false;
                }

                Ok(futures::stream::iter(ids).then(move |(id, sponsored)| {
                    let ok = ok.clone();
                    let client = client.clone();
                    async move {
                        /* be nice! */
                        let sleep = tokio::time::sleep(Duration::from_millis(600));
                        let fut = async {
                            let mut guard = client.lock().await;
                            let real_client = &mut guard;
                            Self::by_id(real_client, id).await
                        };

                        let mut prod = tokio::join!(fut, sleep).0?;
                        /* mark that at least one of the links worked */
                        {
                            let mut guard = ok.lock().await;
                            *guard = true;
                        }

                        prod.sponsored = Some(sponsored);

                        Ok(prod)
                    }
                }))
            }
        });

        stream_stream
            .take_while(|r| futures::future::ready(r.is_ok()))
            .then(|r| futures::future::ready(r.unwrap()))
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use crate::common::Client;

    use super::Product;

    #[tokio::test]
    async fn test_by_id() {
        let mut client = Client::default();

        let prod = Product::by_id(&mut client, 254625474154).await.unwrap();

        assert_eq!(prod.seller.as_ref().unwrap().name, "bellwetherbooks_usa");

        assert!(prod.seller.as_ref().unwrap().feedback.unwrap() > 0.9);
        assert!(prod.seller.as_ref().unwrap().feedback.unwrap() < 1.0);

        assert!(prod.name.contains("Rust Programming Language"));
    }

    #[tokio::test]
    async fn test_search() {
        let products = Product::search("cpu").take(20).collect::<Vec<_>>().await;
        let products = products
            .into_iter()
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();
        let total_products = products.len();
        assert!(total_products >= 16, "total_products = {}", total_products);
        let sponsored = products
            .iter()
            .filter(|p| p.sponsored == Some(true))
            .count();

        assert!(sponsored >= 3, "sponsored = {}", sponsored);

        let amd = products
            .iter()
            .filter(|p| p.name.to_lowercase().contains("amd"))
            .count();
        assert!(amd >= 3, "amd = {}", amd);
    }
}
