mod buffer;
mod config;
mod environment;
mod font;
mod history;
mod icon;
mod identity;
mod logger;
mod network;
mod screen;
mod theme;
mod widget;
mod window;

use config::Config;
use iced::{Subscription, Task, widget::column};
use theme::Theme;
use tokio::runtime;
use widget::Element;

fn main() -> iced::Result {
    iced::daemon(Freecord::initial_setup, Freecord::update, Freecord::view)
        .title(Freecord::title)
        .theme(Freecord::theme)
        .subscription(Freecord::subscription)
        .settings(Freecord::settings())
        .run()
}

enum Screen {
    Welcome(screen::welcome::Welcome),
    Dashboard(screen::dashboard::Dashboard),
}

/// The state of the app.
struct Freecord {
    main_window: window::Window,
    screen: Screen,
    config: Config,
    theme: Theme,
    pane_logs: Vec<logger::Record>,
}

#[derive(Debug)]
enum Message {
    ConfigReloaded(Result<Config, config::Error>),
    Window(window::Id, window::Event),
    Welcome(screen::welcome::Message),
    Dashboard(screen::dashboard::Message),
    Network(network::Message),
    Logging(Vec<logger::Record>),
}

impl Freecord {
    fn new(
        main_window: window::Window,
        config: Result<Config, config::Error>,
        theme: Theme,
        pane_logs: Vec<logger::Record>,
    ) -> (Freecord, Task<Message>) {
        let (config, screen) = match config {
            Ok(config) => (
                config,
                Screen::Dashboard(screen::dashboard::Dashboard::new(&main_window)),
            ),
            Err(config::Error::ConfigMissing) => (
                Config::default(),
                Screen::Welcome(screen::welcome::Welcome::new()),
            ),
            Err(error) => panic!("encountered not yet implemented error handling: {error}"),
        };

        (
            Self {
                main_window,
                screen,
                config,
                theme,
                pane_logs,
            },
            Task::none(),
        )
    }

    fn initial_setup() -> (Freecord, Task<Message>) {
        let is_debug = cfg!(debug_assertions);

        let log_config = Config::load_logs().unwrap_or_default();

        let log_stream = logger::setup(is_debug, log_config).expect("expected logging to be setup");
        log::info!("freecord {} has started", environment::VERSION);
        log::info!("config dir: {:?}", environment::config_dir());
        log::info!("data dir: {:?}", environment::data_dir());

        let config = {
            let rt = runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("expected async runtime");

            rt.block_on(Config::load())
        };

        let theme = Theme::default();

        let (main_window, open_main_window) = window::open(window::Settings {
            exit_on_close_request: false,
            ..window::settings()
        });

        let (freecord, new_task) =
            Self::new(window::Window::new(main_window), config, theme, Vec::new());

        let tasks = vec![
            open_main_window.then(|_| Task::none()),
            Task::stream(log_stream).map(Message::Logging),
            new_task,
        ];

        (freecord, Task::batch(tasks))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ConfigReloaded(config) => self.reload_config(config),
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
                        window::Event::CloseRequested => {
                            log::info!("gracefully shutdown");
                            iced::exit()
                        }
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
                    Some(screen::welcome::Event::RefreshConfig) => {
                        Task::perform(Config::load(), Message::ConfigReloaded)
                    }
                    None => Task::none(),
                }
            }
            Message::Dashboard(message) => {
                let Screen::Dashboard(dashboard) = &mut self.screen else {
                    return Task::none();
                };

                let (event_task, event) = dashboard.update(message, &self.config);

                let task = if let Some(event) = event {
                    match event {
                        screen::dashboard::Event::ConfigReloaded(config) => {
                            self.reload_config(config)
                        }
                    }
                } else {
                    Task::none()
                };

                Task::batch(vec![task, event_task.map(Message::Dashboard)])
            }
            Message::Network(message) => match message {
                network::Message::Identify(event) => {
                    println!("{event:?}");
                    Task::none()
                }
            },
            Message::Logging(records) => {
                self.pane_logs.extend(records);
                Task::none()
            }
        }
    }

    fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        // Main window.
        if window_id == self.main_window.id {
            match &self.screen {
                Screen::Welcome(welcome) => welcome.view().map(Message::Welcome),
                Screen::Dashboard(dashboard) => dashboard
                    .view(&self.pane_logs, &self.config, environment::VERSION)
                    .map(Message::Dashboard),
            }
        // Popped out.
        } else if let Screen::Dashboard(dashboard) = &self.screen {
            dashboard
                .view_popout(window_id, &self.pane_logs, &self.config)
                .map(Message::Dashboard)
        } else {
            column![].into()
        }
    }

    fn title(&self, _window_id: window::Id) -> String {
        String::from("freecord")
    }

    fn theme(&self, _window_id: window::Id) -> Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> Subscription<Message> {
        let subscriptions = vec![
            window::events().map(|(window, event)| Message::Window(window, event)),
            network::listen().map(Message::Network),
        ];

        Subscription::batch(subscriptions)
    }

    fn settings() -> iced::Settings {
        iced::Settings {
            default_font: font::DEFAULT.mono.clone(),
            default_text_size: 13.into(),
            id: None,
            fonts: font::load(),
            antialiasing: false,
        }
    }
}

impl Freecord {
    fn reload_config(&mut self, new_config_result: Result<Config, config::Error>) -> Task<Message> {
        let (freecord, task) = Self::new(
            self.main_window,
            new_config_result,
            self.theme.clone(),
            self.pane_logs.clone(),
        );

        *self = freecord;
        task
    }
}
