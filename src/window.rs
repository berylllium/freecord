use iced::{Point, Size};

pub use iced::window::{
    Event, Id, Position, Settings, close, events, gain_focus, get_position, open,
};

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

#[cfg(target_os = "linux")]
pub fn settings() -> Settings {
    use crate::environment;
    use iced::window;

    Settings {
        platform_specific: window::settings::PlatformSpecific {
            application_id: environment::APPLICATION_ID.to_string(),
            override_redirect: false,
        },
        ..Default::default()
    }
}

#[cfg(target_os = "windows")]
pub fn settings() -> Settings {
    panic!("platform specific window settings for windows have not been defined yet");
}

#[cfg(target_os = "macos")]
pub fn settings() -> Settings {
    panic!("platform specific window settings for macos have not been defined yet");
}
