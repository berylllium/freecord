use serde::{Deserialize, Serialize};

use super::ScrollBar;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Pane {
    pub split_axis: SplitAxis,
    pub scrollbar: ScrollBar,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SplitAxis {
    #[default]
    Horizontal,
    Vertical,
}
