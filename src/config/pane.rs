use iced::widget::pane_grid;
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

impl From<SplitAxis> for pane_grid::Axis {
    fn from(value: SplitAxis) -> Self {
        match value {
            SplitAxis::Horizontal => pane_grid::Axis::Horizontal,
            SplitAxis::Vertical => pane_grid::Axis::Vertical,
        }
    }
}

impl From<pane_grid::Axis> for SplitAxis {
    fn from(value: pane_grid::Axis) -> Self {
        match value {
            pane_grid::Axis::Horizontal => Self::Horizontal,
            pane_grid::Axis::Vertical => Self::Vertical,
        }
    }
}
