use crate::schemas::common::Rating;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum Currency {
    USD,
}

impl Currency {
    pub fn from_abbreviation<S: AsRef<str>>(s: S) -> Option<Self> {
        match s.as_ref().to_ascii_uppercase().as_str() {
            "USD" => Some(Self::USD),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Price {
    pub unit: Currency,
    pub amount: f64,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Seller {
    pub link: Option<String>,
    pub name: Option<String>,
    pub rating: Option<Rating>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Product {
    pub link: Option<String>,
    pub price: Option<Price>,
    pub name: Option<String>,
    pub seller: Option<Seller>,
}
