use std::hash::Hash;

use indexmap::IndexMap;
use iroh::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct Node {
    pub node_id: NodeId,
    /// The nickname of this node, will use node config name if left unspecified.
    pub nickname: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Map(pub IndexMap<String, Node>);

impl Map {
    pub fn empty() -> Self {
        Self(IndexMap::new())
    }
}

impl Hash for Map {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_slice().hash(state);
    }
}
