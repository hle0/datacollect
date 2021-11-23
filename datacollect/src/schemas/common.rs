use serde::{Deserialize, Serialize};

/*
 * Represents a numeric rating from many users.
 */
#[derive(Serialize, Deserialize)]
pub struct Rating {
    pub fraction: f64,
    pub reviewers: Option<u32>,
}
