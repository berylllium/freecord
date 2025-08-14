mod config;
mod environment;
mod font;
mod welcome;
mod window;

use iced::{Element, Subscription, Task, Theme, widget::column};

fn main() -> iced::Result {
    iced::daemon(Freecord::new, Freecord::update, Freecord::view)
        .title(Freecord::title)
        .theme(Freecord::theme)
        .subscription(Freecord::subscription)
        .settings(Freecord::settings())
        .run()
}

enum Screen {
    Welcome(welcome::Welcome),
}

/// The state of the app.
struct Freecord {
    main_window: window::Window,
    screen: Screen,
}

#[derive(Debug)]
enum Message {
    Window(window::Id, window::Event),
    Welcome(welcome::Message),
}

impl Freecord {
    fn new() -> (Freecord, Task<Message>) {
        let (main_window, open_main_window) = window::open(window::Settings {
            exit_on_close_request: false,
            ..Default::default()
        });

        let freecord = Self {
            main_window: window::Window::new(main_window),
            screen: Screen::Welcome(welcome::Welcome::new()),
        };

        (freecord, open_main_window.then(|_| Task::none()))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Window(id, event) => {
                if id == self.main_window.id {
                    match event {
                        window::Event::Unfocused => {
                            self.main_window.focused = false;
                            Task::none()
                        }
                        window::Event::Focused => {
                            self.main_window.focused = true;
                            Task::none()
                        }
                        window::Event::Opened { position, size } => {
                            self.main_window.initialize(position, size);
                            Task::none()
                        }
                        window::Event::CloseRequested => iced::exit(),
                        _ => Task::none(),
                    }
                } else {
                    Task::none()
                }
            }
            Message::Welcome(message) => {
                let Screen::Welcome(welcome) = &mut self.screen else {
                    return Task::none();
                };

                match welcome.update(message) {
                    Some(_) => Task::none(),
                    None => Task::none(),
                }
            }
        }
    }

    fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        // Main window.
        if window_id == self.main_window.id {
            match &self.screen {
                Screen::Welcome(welcome) => welcome.view().map(Message::Welcome),
            }
        } else {
            column![].into()
        }
    }

    fn title(&self, _window_id: window::Id) -> String {
        String::from("freecord")
    }

    fn theme(&self, _window_id: window::Id) -> Theme {
        Theme::GruvboxDark
    }

    fn subscription(&self) -> Subscription<Message> {
        let subscriptions =
            vec![window::events().map(|(window, event)| Message::Window(window, event))];

        Subscription::batch(subscriptions)
    }

    fn settings() -> iced::Settings {
        iced::Settings {
            default_font: font::DEFAULT.mono.clone(),
            default_text_size: 16.into(),
            id: None,
            fonts: font::load(),
            antialiasing: false,
        }
    }
}
