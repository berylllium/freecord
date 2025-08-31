use iced::{
    Length, Task, alignment,
    widget::{button, column, container, row, stack, text, vertical_space},
};

use crate::{
    config::{self, Config},
    icon,
    theme::{self},
    widget::{Column, Element, Text, context_menu::context_menu},
};

pub struct Sidebar {}

#[derive(Debug, Clone)]
pub enum Message {
    OpenLogs,
    OpenConfigFile,
    ReloadConfig,
    ConfigReloaded(Result<Config, config::Error>),
    SwitchToIdentity,
}

#[derive(Debug, Clone)]
pub enum Event {
    OpenLogs,
    OpenConfigFile,
    ConfigReloaded(Result<Config, config::Error>),
    SwitchToIdentity,
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
            Message::OpenLogs => (Task::none(), Some(Event::OpenLogs)),
            Message::SwitchToIdentity => (Task::none(), Some(Event::SwitchToIdentity)),
        }
    }

    pub fn view<'a>(
        &'a self,
        nodes: &'a config::NodeMap,
        version: &'static str,
    ) -> Element<'a, Message> {
        let contacts = container(Column::from_iter(
            nodes
                .0
                .iter()
                .map(|(name, config)| Self::node_entry(name, config)),
        ))
        .padding([8, 4])
        .height(Length::Fill);

        let menu_button = self.menu_button(version);

        column![contacts, menu_button].into()
    }

    pub fn node_entry<'a>(node_name: &'a String, node: &'a config::Node) -> Element<'a, Message> {
        let name = match &node.nickname {
            Some(nickname) => nickname.as_str(),
            None => node_name.as_str(),
        };

        let entry = row![icon::globe(), name]
            .spacing(4)
            .align_y(alignment::Vertical::Center);

        button(entry)
            .padding(4)
            .style(|theme, status| theme::button::secondary(theme, status, false))
            .width(Length::Shrink)
            .height(Length::Shrink)
            .into()
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
                    Menu::OpenLogs => context_button(text("Logs"), icon::logs(), Message::OpenLogs),
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
                    Menu::SwitchToIdentity => context_button(
                        text("Identity"),
                        icon::identity(),
                        Message::SwitchToIdentity,
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
    OpenLogs,
    SwitchToIdentity,
}

impl Menu {
    fn list() -> Vec<Self> {
        vec![
            Self::Version,
            Self::OpenLogs,
            Self::OpenConfigFile,
            Self::ReloadConfigFile,
            Self::SwitchToIdentity,
        ]
    }
}
