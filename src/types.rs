use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct ValueWithExpiry {
    pub value: String,
    pub expiry: Option<Duration>,
    pub insert_time: Instant,
}

pub type SharedState = Arc<Mutex<HashMap<String, ValueWithExpiry>>>;
