use iced::{
    Length, alignment,
    theme::Palette,
    widget::{button, column, container, row, space, text},
};

use crate::{config::Config, environment, theme, widget::Element};

#[derive(Clone)]
pub struct Welcome;

#[derive(Clone, Debug)]
pub enum Message {
    OpenConfigDirectory,
    RefreshConfig,
}

pub enum Event {
    RefreshConfig,
}

impl Welcome {
    pub fn new() -> Self {
        Config::create_initial_config();

        Self
    }

    pub fn update(&mut self, message: Message) -> Option<Event> {
        match message {
            Message::OpenConfigDirectory => {
                let _ = open::that_detached(Config::config_dir());

                None
            }
            Message::RefreshConfig => Some(Event::RefreshConfig),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let config_dir = String::from(Config::config_dir().to_string_lossy());

        let config_button = button(
            container(text(config_dir))
                .width(250)
                .align_x(alignment::Horizontal::Center),
        )
        .style(|theme, state| theme::button::secondary(theme, state, false))
        .padding([5, 20])
        .width(Length::Shrink)
        .on_press(Message::OpenConfigDirectory);

        let refresh_button = button(
            container(text("Reload configuration."))
                .width(250)
                .align_x(alignment::Horizontal::Center),
        )
        .style(|theme, state| theme::button::secondary(theme, state, false))
        .padding([5, 20])
        .width(Length::Shrink)
        .on_press(Message::RefreshConfig);

        let content = column![]
            .spacing(1)
            .push(text("Welcome to Freecord!"))
            .push(space::vertical().height(4))
            .push(text("Freecord is configured using a config file."))
            .push(row![
                text("You can find the "),
                text(environment::CONFIG_FILE_NAME).style(|_| text::Style {
                    color: Some(Palette::GRUVBOX_DARK.warning)
                }),
                text(" file at:"),
            ])
            .push(space::vertical().height(8))
            .push(config_button)
            .push(space::vertical().height(8))
            .push(refresh_button)
            .align_x(alignment::Horizontal::Center);

        column![
            container(content)
                .align_x(alignment::Horizontal::Center)
                .align_y(alignment::Vertical::Center)
                .width(Length::Fill)
                .height(Length::Fill),
            text(environment::VERSION).size(12).color({
                let bg = Palette::GRUVBOX_DARK.background;

                [
                    (bg.r * 1.8).min(1.0),
                    (bg.g * 1.8).min(1.0),
                    (bg.b * 1.8).min(1.0),
                ]
            })
        ]
        .into()
    }
}
