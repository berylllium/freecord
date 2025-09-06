use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
#[serde(default)]
pub struct Network {
    pub reconnect_delay: u64,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            reconnect_delay: 10,
        }
    }
}
