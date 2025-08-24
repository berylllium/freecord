use iced::{Point, Size};

pub use iced::window::{Event, Id, Settings, close, events, gain_focus, open};

#[derive(Debug, Clone, Copy)]
pub struct Window {
    pub id: Id,
    pub focused: bool,
    pub state: WindowState,
}
#[derive(Debug, Clone, Copy, Default)]
pub enum WindowState {
    #[default]
    Uninitialized,
    Initialized {
        position: Option<Point>,
        size: Size,
    },
}

impl Window {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            focused: false,
            state: WindowState::default(),
        }
    }

    pub fn initialize(&mut self, position: Option<Point>, size: Size) {
        match self.state {
            WindowState::Uninitialized => self.state = WindowState::Initialized { position, size },
            WindowState::Initialized {
                position: _,
                size: _,
            } => panic!("Cannot initialize an already initialized window."),
        }
    }
}
