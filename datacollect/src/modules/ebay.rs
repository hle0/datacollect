use async_trait::async_trait;
use kuchiki::traits::TendrilSink;
use lazy_static::lazy_static;
use reqwest::Client;

use crate::common::{DataProcessor, Depth};
use crate::schemas::common::Rating;
use crate::schemas::money::{Currency, Price, Product, Seller};

pub struct EbayItemSource {
    pub client: Client,
}

impl EbayItemSource {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            client: Client::builder().build()?,
        })
    }
}

#[async_trait]
impl DataProcessor<Product, u64> for EbayItemSource {
    async fn process(&mut self, id: u64, _depth: Depth) -> anyhow::Result<Product> {
        lazy_static! {
            static ref RE_USR: regex::Regex =
                regex::Regex::new(r"https://(?:www\.)?ebay\.com/usr/([a-zA-Z0-9_\-]+)(?:\?.*)?")
                    .unwrap();
            static ref RE_PERCENT: regex::Regex =
                regex::Regex::new(r"([0-9]+(?:\.[0-9]+)?)%").unwrap();
        };

        let link = format!("https://www.ebay.com/itm/foo/{}", id);

        let response = self.client.get(link.clone()).send().await?;
        let text = response.text().await?;
        let document = kuchiki::parse_html().one(text);

        let title: Option<String> = try {
            document
                .select_first("#itemTitle")
                .ok()?
                .as_node()
                .children()
                .map(|node| {
                    let s = node.as_text()?.borrow();
                    if s.trim().is_empty() {
                        None
                    } else {
                        Some(s.trim().to_string())
                    }
                })
                .find(|o| o.is_some())?
                .unwrap()
        };

        let seller: Option<Seller> = try {
            let seller_info = document.select_first(".si-content").ok()?;
            let name: String = seller_info
                .as_node()
                .select("a[href]")
                .ok()?
                .map(|a| {
                    let href = {
                        let attributes = a.attributes.borrow();
                        attributes.get("href")?.to_string()
                    };
                    let username = RE_USR.captures(href.as_str())?.get(1)?.as_str().to_string();
                    Some(username)
                })
                .find(|o| o.is_some())?
                .unwrap();
            let link = format!("https://www.ebay.com/usr/{}", name);
            let rating: Option<Rating> = try {
                let text = seller_info
                    .as_node()
                    .select_first("#si-fb")
                    .ok()?
                    .as_node()
                    .text_contents();
                let percent = RE_PERCENT.captures(text.as_str())?.get(1)?.as_str();
                Rating {
                    fraction: percent.parse::<f64>().ok()? * 0.01,
                    reviewers: None,
                }
            };

            Seller {
                link: Some(link),
                name: Some(name),
                rating,
            }
        };

        let price: Option<Price> = try {
            let main_price = document.select_first(".mainPrice").ok()?;

            let unit = Currency::from_abbreviation({
                let price_currency = main_price
                    .as_node()
                    .select_first("[itemprop=priceCurrency]")
                    .ok()?;
                let attributes = price_currency.attributes.borrow();
                attributes.get("content")?.to_string()
            })?;

            let amount = {
                let price_prop = main_price.as_node().select_first("[itemprop=price]").ok()?;
                let attributes = price_prop.attributes.borrow();
                attributes.get("content")?.parse::<f64>().ok()?
            };

            Price { unit, amount }
        };

        Ok(Product {
            link: Some(link),
            name: title,
            price,
            seller,
        })
    }
}
