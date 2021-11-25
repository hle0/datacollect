use anyhow::Context;
use kuchiki::traits::TendrilSink;
use lazy_static::lazy_static;
use serde::Serialize;

use crate::common::{Client, Currency};

#[derive(Serialize)]
pub struct Seller {
    pub name: String,
    pub feedback: Option<f64>,
}

#[derive(Serialize)]
pub struct Product {
    pub name: String,
    pub seller: Option<Seller>,
    pub price: Option<(Currency, f64)>,
}

impl Product {
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
                    .map(|node| {
                        let s = node.as_text()?.borrow();
                        if s.trim().is_empty() {
                            None
                        } else {
                            Some(s.trim().to_string())
                        }
                    })
                    .find(Option::is_some)
                    .context("trying to get title")?
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
                    .find(Option::is_some)?
                    .unwrap();
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

            let price: Option<(Currency, f64)> = try {
                /* TODO: work on sold eBay listings (e.g. 255166134948) */
                let main_price = document
                    .select_first(".mainPrice")
                    .or_else(|_| document.select_first(".vi-price"))
                    .ok()?;

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

                (unit, amount)
            };

            Self {
                name,
                price,
                seller,
            }
        };

        product
    }
}

#[cfg(test)]
mod tests {
    use crate::common::Client;

    use super::Product;

    #[tokio::test]
    async fn test_processor() {
        let mut client = Client::default();

        let prod = Product::by_id(&mut client, 254625474154).await.unwrap();

        assert_eq!(prod.seller.as_ref().unwrap().name, "bellwetherbooks_usa");

        assert!(prod.seller.as_ref().unwrap().feedback.unwrap() > 0.9);
        assert!(prod.seller.as_ref().unwrap().feedback.unwrap() < 1.0);

        assert!(prod.name.contains("Rust Programming Language"));
    }
}
