use iced::{
    Length, Task,
    widget::{button, column, container, row, stack, text, vertical_space},
};

use crate::{
    config::{self, Config},
    icon,
    theme::{self},
    widget::{Element, Text, context_menu::context_menu},
};

pub struct Sidebar {}

#[derive(Debug, Clone)]
pub enum Message {
    OpenConfigFile,
    ReloadConfig,
    ConfigReloaded(Result<Config, config::Error>),
}

#[derive(Debug, Clone)]
pub enum Event {
    OpenConfigFile,
    ConfigReloaded(Result<Config, config::Error>),
}

impl Sidebar {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update<'a>(&'a self, message: Message) -> (Task<Message>, Option<Event>) {
        match message {
            Message::OpenConfigFile => (Task::none(), Some(Event::OpenConfigFile)),
            Message::ReloadConfig => (Task::perform(Config::load(), Message::ConfigReloaded), None),
            Message::ConfigReloaded(config) => (Task::none(), Some(Event::ConfigReloaded(config))),
        }
    }

    pub fn view<'a>(&'a self, version: &'static str) -> Element<'a, Message> {
        let contacts = vertical_space().height(Length::Fill);

        let menu_button = self.menu_button(version);

        column![contacts, menu_button].into()
    }

    pub fn menu_button<'a>(&self, version: &'static str) -> Element<'a, Message> {
        let base = button(icon::menu()).padding(5).width(Length::Shrink);

        let menu = Menu::list();

        if menu.is_empty() {
            base.into()
        } else {
            stack![context_menu(base, menu, move |menu, length| {
                let context_button = |title: Text<'a>, icon: Text<'a>, message: Message| {
                    button(
                        row![icon.width(Length::Fixed(12.0)), title]
                            .spacing(8)
                            .align_y(iced::Alignment::Center),
                    )
                    .width(length)
                    .padding(5)
                    .on_press(message)
                    .into()
                };

                match menu {
                    Menu::Version => container(
                        text(format!("Freecord ({})", version)).style(theme::text::secondary),
                    )
                    .padding(5)
                    .into(),
                    Menu::OpenConfigFile => context_button(
                        text("Open config file"),
                        icon::config_file(),
                        Message::OpenConfigFile,
                    ),
                    Menu::ReloadConfigFile => context_button(
                        text("Reload config file"),
                        icon::refresh(),
                        Message::ReloadConfig,
                    ),
                }
            })]
            .into()
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Menu {
    Version,
    OpenConfigFile,
    ReloadConfigFile,
}

impl Menu {
    fn list() -> Vec<Self> {
        vec![Self::Version, Self::OpenConfigFile, Self::ReloadConfigFile]
    }
}
