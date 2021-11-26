use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::common::Client;

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub event_action: String,
    pub event_actor: Option<String>,
    pub event_date: DateTime<Utc>,
}

#[derive(Deserialize, Serialize)]
pub struct DomainRecord {
    /* TODO: add more fields. see: https://datatracker.ietf.org/doc/html/rfc7483#section-4 */
    pub events: Vec<Event>,
}

impl DomainRecord {
    pub async fn get(client: &mut Client<false>, domain: &str) -> anyhow::Result<Option<Self>> {
        let res = client
            .0
            .get(format!("https://rdap.org/domain/{}", domain))
            .send()
            .await?;
        if res.status() == 404 {
            Ok(None)
        } else {
            Ok(Some(res.json().await?))
        }
    }

    fn events_in_time_backwards(&self) -> Vec<Event> {
        let mut events = self.events.clone();
        events.sort_by_key(|e| -e.event_date.timestamp_millis());
        events
    }

    pub fn is_locked_at(&self, now: &DateTime<Utc>) -> bool {
        self.events_in_time_backwards()
            .iter()
            .filter(|e| &e.event_date < now)
            .find_map(|e| match e.event_action.as_str() {
                "locked" => Some(true),
                "unlocked" => Some(false),
                _ => None,
            })
            .unwrap_or(false)
    }

    pub fn is_registered_at(&self, now: &DateTime<Utc>) -> bool {
        self.events_in_time_backwards()
            .iter()
            .filter(|e| &e.event_date < now)
            .find_map(|e| match e.event_action.as_str() {
                "reregistration" | "registration" | "reinstantiation" | "transfer" => Some(true),
                "expiration" | "deletion" => Some(false),
                _ => None,
            })
            .unwrap_or(false)
    }

    pub fn is_buyable_at(&self, now: &DateTime<Utc>) -> bool {
        // presumably, locked domains cannot be bought or expire. TODO: figure out definitively
        // also TODO: there doesn't seem to be an easy source to find which TLDs are unrestricted.
        //      .gov and .com are treated the same way.
        !(self.is_registered_at(now) || self.is_locked_at(now))
    }
}

#[cfg(test)]
mod tests {
    use hex::ToHex;

    use super::DomainRecord;

    #[tokio::test]
    async fn test_google() {
        let record = DomainRecord::get(&mut Default::default(), "google.com")
            .await
            .unwrap()
            .unwrap();
        let now = chrono::Utc::now();
        assert_eq!(record.is_locked_at(&now), false);
        assert_eq!(record.is_registered_at(&now), true);
        assert_eq!(record.is_buyable_at(&now), false);
    }

    #[tokio::test]
    async fn test_random() {
        let domain = format!("{}.net", rand::random::<[u8; 10]>().encode_hex::<String>());
        let record = DomainRecord::get(&mut Default::default(), domain.as_str())
            .await
            .unwrap();
        assert_eq!(record.is_none(), true);
    }
}
