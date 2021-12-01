use anyhow::{anyhow, bail, Context};
use serde::{de::Visitor, Deserialize, Serialize};
use serde_with::{DeserializeAs, DeserializeFromStr, SerializeDisplay};
use std::{convert::TryFrom, fmt::Display, marker::PhantomData, str::FromStr};

/// A currency - some type of money.
#[derive(SerializeDisplay, DeserializeFromStr)]
pub enum Currency {
    USD,
}

impl Currency {
    /// Given a price with a currency symbol and an amount, try to extract a [`Currency`] from the symbol.
    pub fn from_price<S: AsRef<str>>(s: S) -> Option<Self> {
        s.as_ref()
            .split(|c: char| c.is_whitespace() || c.is_numeric())
            .find_map(|s| {
                (!s.is_empty())
                    .then(|| Self::from_abbreviation(s))
                    .flatten()
            })
    }

    /// Given an abbreviation/symbol, try to return the corresponding [`Currency`].
    /// Only considers alphabetic characters - `$` is filtered out, for example.
    pub fn from_abbreviation<S: AsRef<str>>(s: S) -> Option<Self> {
        match s
            .as_ref()
            .chars()
            .flat_map(char::to_lowercase)
            .filter(|c| c.is_alphabetic())
            .collect::<String>()
            .as_str()
        {
            "" | "us" | "usd" => Some(Self::USD),
            _ => None,
        }
    }
}

impl FromStr for Currency {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Self::from_abbreviation(s) {
            Some(thing) => Ok(thing),
            None => bail!("no such abbreviation"),
        }
    }
}

impl Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::USD => "USD",
            }
        )
    }
}

/// Convert something like "$312.03" to 312.03
///
/// ## Example
/// ```txt
/// "$312.03" -> 312.03
/// "312.03"  -> 312.03
/// "312"     -> 312.0  
/// "312.009" -> 312.009
/// ```
pub(crate) fn parse_dollars<T: AsRef<str>>(s: T) -> Option<f64> {
    s.as_ref()
        .chars()
        .filter(|c| c.is_numeric() || *c == '.')
        .collect::<String>()
        .parse::<f64>()
        .ok()
}

/// Currency ([`Currency`]), and some amount of it ([`f64`]).
/// Currently, money with no [`Currency`] is assumed to be USD.
#[derive(Serialize, Deserialize)]
pub struct Money(Currency, f64);

impl FromStr for Money {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cur = Currency::from_price(s).unwrap_or(Currency::USD);
        let price = s
            .split(char::is_whitespace)
            .find_map(|s| (!s.is_empty()).then(|| parse_dollars(s)).flatten())
            .ok_or_else(|| anyhow!("failed to find price"))?;
        Ok(Self(cur, price))
    }
}

impl TryFrom<crate::schema_org::Scope> for Money {
    type Error = anyhow::Error;
    fn try_from(scope: crate::schema_org::Scope) -> anyhow::Result<Self> {
        let price = scope
            .get_value("price")
            .context("could not get price of item through schema.org microdata")?;
        if let Some(cur) = scope
            .get_value("priceCurrency")
            .and_then(Currency::from_abbreviation)
        {
            let dollars = parse_dollars(price).context("could not parse currency amount")?;
            Ok(Self(cur, dollars))
        } else {
            Self::from_str(&price)
        }
    }
}

/// Ignore commas when parsing number formats.
/// e.g. 13,096,340.3 -> 13096340.3
pub struct IgnoreComma<T>
where
    T: FromStr,
{
    _t: PhantomData<T>,
}

impl<'de, T> DeserializeAs<'de, T> for IgnoreComma<T>
where
    T: FromStr,
{
    fn deserialize_as<D>(deserializer: D) -> Result<T, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Helper<TT>
        where
            TT: FromStr,
        {
            _tt: PhantomData<TT>,
        }

        impl<'de, TT> Visitor<'de> for Helper<TT>
        where
            TT: FromStr,
        {
            type Value = TT;

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                TT::from_str(v.replace(',', "").as_str())
                    .map_err(|_| E::custom("format error while parsing in IgnoreComma"))
            }

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                /* TODO: I really wish the error message could be implemented better */
                formatter.write_fmt(format_args!("a FromStr (probably number), ignoring commas"))
            }
        }

        deserializer.deserialize_str(Helper::<T> { _tt: PhantomData })
    }
}

/// A wrapped [`reqwest::Client`].
/// Some scrapers require cookies, while some don't need cookies.
/// This struct takes advantage of Rust's static typing to make sure
/// that scrapers that require cookies are never given a [`reqwest::Client`]
/// that does not have a cookie jar.
pub struct Client<const COOKIES: bool>(pub reqwest::Client);

impl<const COOKIES: bool> Default for Client<COOKIES> {
    fn default() -> Self {
        Self(
            reqwest::Client::builder()
                .cookie_store(COOKIES)
                .build()
                .unwrap(),
        )
    }
}

/// Checks if all the characters in `needle` can be found in `haystack` in the same order.
///
/// Some platforms like to obfuscate certain visible text fields from bots.
/// For example, eBay makes it more difficult to identify sponsored items by using several
/// `display: none;` fields.
/// A human may read the text as `Sponsored`, while a robot may read it as `ddSQpOonhsortied`,
/// because the extra characters are actually their own `<span>`'s that are not displayed in a browser.
/// This is presumably to make it harder to automatically block sponsored listings from appearing using extensions.
pub(crate) fn has_hidden_word(needle: &str, haystack: &str) -> bool {
    haystack
        .chars()
        .fold(needle, |acc, c| {
            if acc.is_empty() {
                /* we've finished! */
                acc
            } else {
                /* check if the first chars match. */
                if c == acc.chars().next().unwrap() {
                    /* they matched! now just chop off the first char of the needle */
                    &acc[1..]
                } else {
                    /* they didn't match. this char must have been added to fool us! */
                    acc
                }
            }
        })
        .is_empty()
}

#[cfg(test)]
mod tests {
    use super::has_hidden_word;

    use super::parse_dollars;

    fn roughly_equal(a: f64, b: f64) -> bool {
        if a == b {
            true
        } else if ((a > 0.0) && (b < 0.0)) || ((a < 0.0) && (b > 0.0)) {
            false
        } else if ((a == 0.0) && (b != 0.0)) || ((a != 0.0) && (b == 0.0)) {
            false
        } else {
            fn dif(x: f64, y: f64) -> f64 {
                (x.abs().ln() - y.abs().ln()).abs()
            }

            dif(a, b) <= dif(1.0, 1.00001)
        }
    }

    #[test]
    fn test_roughly_equal() {
        assert!(roughly_equal(0.0, 0.0));
        assert!(roughly_equal(0.02, 0.02));
        assert!(roughly_equal(0.02, 0.02000001));
        assert!(roughly_equal(0.1 + 0.2, 0.3));
        assert!(roughly_equal(0.1 - 0.2, 0.2 - 0.3));
        assert!(roughly_equal(4000000.0, 4000000.2));
        assert!(roughly_equal(-4000000.0, -4000000.2));

        assert!(!roughly_equal(0.02, 0.03));
        assert!(!roughly_equal(0.00002, 0.00003));
        assert!(!roughly_equal(0.0, 0.00003));
        assert!(!roughly_equal(0.00002, 0.00003));
        assert!(!roughly_equal(1000.0, 1001.0));
        assert!(!roughly_equal(2.0, -2.0));
    }

    #[test]
    fn test_parse_dollars() {
        assert_eq!(parse_dollars("$312.04").unwrap(), 312.04);
        assert_eq!(parse_dollars("8.8.4.4"), None);
        assert_eq!(parse_dollars("42").unwrap(), 42.00);
        assert_eq!(parse_dollars("$42.567").unwrap(), 42.567);
    }

    #[test]
    fn test_has_hidden_word() {
        assert_eq!(has_hidden_word("cookie", "cooOOOkie"), true);
        assert_eq!(has_hidden_word("cookie", "cookie"), true);
        assert_eq!(has_hidden_word("cookie", "423TGRcoAFoGRkHiDSDGRTe"), true);
        assert_eq!(
            has_hidden_word("baking cookies", "some cookie baking"),
            false
        );
        assert_eq!(has_hidden_word("candy canes", "candy"), false);
    }
}
