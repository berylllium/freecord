use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(default)]
pub struct Network {
    pub relay_address: Option<String>,
    pub reconnect_delay: u64,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            relay_address: None,
            reconnect_delay: 10,
        }
    }
}
