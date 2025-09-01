pub mod logs;
pub mod network;
pub mod node;
pub mod pane;

use std::path::PathBuf;

pub use logs::Logs;
pub use network::Network;
pub use node::{Map as NodeMap, Node};
pub use pane::Pane;
use serde::{Deserialize, Serialize};

use crate::{environment, modal};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub logs: Logs,
    pub font: Font,
    pub pane: Pane,
    pub network: Network,
    pub nodes: NodeMap,
}

impl Config {
    pub fn config_dir() -> PathBuf {
        let dir = environment::config_dir();

        if !dir.exists() {
            std::fs::create_dir_all(dir.as_path())
                .expect("expected permissions to create config folder");
        }

        dir
    }

    pub fn path() -> PathBuf {
        Self::config_dir().join(environment::CONFIG_FILE_NAME)
    }

    pub async fn load() -> Result<Self, Error> {
        use tokio::fs;

        let path = Self::path();
        if !path.try_exists()? {
            return Err(Error::ConfigMissing);
        }

        let content = fs::read_to_string(path)
            .await
            .map_err(|e| Error::LoadConfigFile(e.to_string()))?;

        let config =
            toml::Deserializer::parse(content.as_ref()).map_err(|e| Error::Parse(e.to_string()))?;

        let config = serde_ignored::deserialize(config, |ignored| {
            log::warn!("[config.toml] Ignoring unknown setting: {ignored}")
        })
        .map_err(|e| Error::Parse(e.to_string()))?;

        Ok(config)
    }

    pub fn load_logs() -> Option<Logs> {
        #[derive(Default, Deserialize)]
        #[serde(default)]
        pub struct Configuration {
            pub logs: Logs,
        }

        let path = Self::path();
        let content = std::fs::read_to_string(path).ok()?;

        let Configuration { logs } = toml::from_str(content.as_ref()).ok()?;

        Some(logs)
    }

    pub fn create_initial_config() {
        let config_file_path = Self::path();
        if config_file_path.exists() {
            return;
        }

        let default = Self::default();

        let config_bytes = toml::to_string_pretty(&default)
            .expect("expected valid default config serialization")
            .into_bytes();

        let _ = std::fs::write(config_file_path, config_bytes);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Font {
    pub family: Option<String>,
    pub size: Option<u8>,
}

impl Default for Font {
    fn default() -> Self {
        Self {
            family: Some("Iosevka Term".to_string()),
            size: Some(12),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScrollBar {
    pub width: u32,
    pub scroller_width: u32,
}

impl Default for ScrollBar {
    fn default() -> Self {
        Self {
            width: 5,
            scroller_width: 5,
        }
    }
}

#[derive(Debug, thiserror::Error, Clone)]
pub enum Error {
    #[error("config could not be read: {0}")]
    LoadConfigFile(String),
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Parse(String),
    #[error("config does not exist")]
    ConfigMissing,
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<Error> for modal::Error {
    fn from(value: Error) -> Self {
        Self {
            title: "Configuration error".to_string(),
            message: value.to_string(),
        }
    }
}
