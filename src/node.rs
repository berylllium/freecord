use std::sync::Arc;

use indexmap::IndexMap;
use iroh::{NodeId, endpoint::RemoteInfo};

use crate::{config, network::node::Sender};

#[derive(Debug)]
pub enum ConnectionState {
    Disconnected,
    Connected {
        direction: Direction,
        node_info: RemoteInfo,
        sender: Sender,
    },
}

#[derive(Debug)]
pub enum Direction {
    Outbound,
    Inbound,
}

#[derive(Debug)]
pub struct Node {
    pub name: String,
    pub nickname: Option<String>,
    pub connection_state: ConnectionState,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            name: String::default(),
            nickname: None,
            connection_state: ConnectionState::Disconnected,
        }
    }
}

#[derive(Debug)]
pub struct Map(IndexMap<NodeId, Node>);

impl Map {
    pub fn new(nodes: config::NodeMap) -> Self {
        Self(IndexMap::from_iter(nodes.0.into_iter().map(
            |(name, node)| {
                (
                    node.node_id,
                    Node {
                        name: name,
                        nickname: node.nickname,
                        ..Default::default()
                    },
                )
            },
        )))
    }

    pub fn connected(&mut self, node_id: NodeId, connection_state: ConnectionState) {
        if let Some(node) = self.0.get_mut(&node_id) {
            node.connection_state = connection_state;
        } else {
            self.0.insert(
                node_id,
                Node {
                    name: "Unknown".to_string(),
                    connection_state,
                    ..Default::default()
                },
            );
        }
    }

    pub fn disconnected(&mut self, node_id: NodeId) {
        if let Some(node) = self.0.get_mut(&node_id) {
            node.connection_state = ConnectionState::Disconnected;
        }
    }
}

impl Default for Map {
    fn default() -> Self {
        Self(IndexMap::new())
    }
}
