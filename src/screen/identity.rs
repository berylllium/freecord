use iced::{
    Background, Border, Color, Length, Task, alignment, clipboard,
    widget::{
        button, center, column, container, horizontal_rule, horizontal_space, row, text,
        vertical_space,
    },
};

use crate::{
    identity,
    theme::{self, Theme},
    widget::{Button, Container, Element},
};

pub struct Identity {}

#[derive(Debug, Clone)]
pub enum Message {
    GenerateSecretKey,
    DeleteSecretKey,
    CopyPrivatePem,
    CopyPublicPem,
}

#[derive(Debug, Clone)]
pub enum Event {
    GenerateSecretKey,
    DeleteSecretKey,
}

impl Identity {
    pub fn new() -> Self {
        Self {}
    }

    pub fn update<'a>(
        &mut self,
        message: Message,
        identity: &identity::Identity,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::GenerateSecretKey => (Task::none(), Some(Event::GenerateSecretKey)),
            Message::DeleteSecretKey => (Task::none(), Some(Event::DeleteSecretKey)),
            Message::CopyPrivatePem => {
                if let Some(keys) = &identity.keys {
                    (
                        keys.private_key_pem()
                            .map_or(Task::none(), |pem| clipboard::write(pem.to_string())),
                        None,
                    )
                } else {
                    (Task::none(), None)
                }
            }
            Message::CopyPublicPem => {
                if let Some(keys) = &identity.keys {
                    (
                        keys.public_key_pem()
                            .map_or(Task::none(), |pem| clipboard::write(pem)),
                        None,
                    )
                } else {
                    (Task::none(), None)
                }
            }
        }
    }

    pub fn view<'a>(&self, identity: &identity::Identity) -> Element<'a, Message> {
        let keypair = {
            fn create_status<'a>(found: bool) -> Container<'a, Message> {
                container(text(if found { "found" } else { "not found" }))
                    .style(move |theme: &Theme| container::Style {
                        background: Some(Background::Color(if found {
                            theme.styles.general.background_success
                        } else {
                            theme.styles.general.background_failure
                        })),
                        border: Border {
                            radius: 4.0.into(),
                            width: 1.0,
                            color: Color::TRANSPARENT,
                        },
                        ..theme::container::general(theme)
                    })
                    .padding([0, 4])
            }

            let keys_exist = identity.keys.is_some();

            let secret_key_status = create_status(keys_exist);
            let public_key_status = create_status(keys_exist);

            let generate_secret_key_button = button(text("Generate"))
                .on_press(Message::GenerateSecretKey)
                .style(|theme, status| theme::button::secondary(theme, status, false));

            let delete_secret_key_button = button(text("Delete"))
                .on_press(Message::DeleteSecretKey)
                .style(|theme, status| theme::button::dangerous(theme, status, false));

            column![
                text("Keypair"),
                vertical_space().height(4),
                row![
                    column![
                        text("Secret Key"),
                        secret_key_status,
                        if keys_exist {
                            Some(
                                button(text("Copy PEM"))
                                    .on_press(Message::CopyPrivatePem)
                                    .style(|theme, status| {
                                        theme::button::secondary(theme, status, false)
                                    }),
                            )
                        } else {
                            None
                        },
                        if keys_exist {
                            delete_secret_key_button
                        } else {
                            generate_secret_key_button
                        },
                    ]
                    .width(Length::Fill)
                    .align_x(alignment::Horizontal::Center)
                    .spacing(4),
                    column![
                        text("Public Key"),
                        public_key_status,
                        if keys_exist {
                            Some(
                                button(text("Copy PEM"))
                                    .on_press(Message::CopyPublicPem)
                                    .style(|theme, status| {
                                        theme::button::secondary(theme, status, false)
                                    }),
                            )
                        } else {
                            None
                        }
                    ]
                    .width(Length::Fill)
                    .align_x(alignment::Horizontal::Center)
                    .spacing(4)
                ]
                .padding([0, 64])
            ]
        };

        let content = column![
            center(text("Identity").size(20))
                .height(Length::Shrink)
                .width(Length::Fill),
            vertical_space().height(8),
            horizontal_rule(1),
            vertical_space().height(4),
            keypair
        ];

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8)
            .into()
    }
}
