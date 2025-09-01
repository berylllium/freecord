mod buffer;
mod config;
mod environment;
mod font;
mod history;
mod icon;
mod identity;
mod logger;
mod modal;
mod network;
mod screen;
mod theme;
mod widget;
mod window;

use clap::Parser;
use config::Config;
use futures::channel::mpsc;
use iced::{Subscription, Task, advanced::subscription, widget::column};
use identity::Identity;
use modal::Modal;
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
    Identity(screen::identity::Identity),
    Dashboard(screen::dashboard::Dashboard),
}

/// The state of the app.
struct Freecord {
    main_window: window::Window,
    screen: Screen,
    modal: Option<Modal>,
    config: Config,
    /// The current swarm stream state. A value of none will drop the stream.
    swarm_stream: Option<network::swarm::Stream>,
    swarm_sender: Option<mpsc::Sender<network::swarm::Input>>,
    identity: Identity,
    theme: Theme,
    pane_logs: Vec<logger::Record>,
    opts: Opts,
}

#[derive(Debug)]
enum Message {
    ConfigReloaded(Result<Config, config::Error>),
    Window(window::Id, window::Event),
    Welcome(screen::welcome::Message),
    Identity(screen::identity::Message),
    Dashboard(screen::dashboard::Message),
    Modal(modal::Message),
    Network(network::swarm::Message),
    Logging(Vec<logger::Record>),
}

impl Freecord {
    fn new(
        main_window: window::Window,
        config: Result<Config, config::Error>,
        identity: Identity,
        theme: Theme,
        pane_logs: Vec<logger::Record>,
        opts: Opts,
    ) -> (Freecord, Task<Message>) {
        let mut modal = None;

        let (mut config, screen) = match config {
            Ok(config) => (
                config,
                Screen::Dashboard(screen::dashboard::Dashboard::new(&main_window)),
            ),
            Err(config::Error::ConfigMissing) => (
                Config::default(),
                Screen::Welcome(screen::welcome::Welcome::new()),
            ),
            Err(error) => {
                Self::push_modal_error(&mut modal, error.into());
                (
                    Config::default(),
                    Screen::Welcome(screen::welcome::Welcome::new()),
                )
            }
        };

        let swarm_stream = if config.network.relay_address.is_some() {
            if let Some(keys) = identity.keys.clone() {
                Some(network::swarm::Stream {
                    keys: keys,
                    config: config.network.clone(),
                    nodes: config.nodes.clone(),
                })
            } else {
                None
            }
        } else {
            Self::push_modal_error(
                &mut modal,
                modal::Error::new(
                    "Networking error",
                    "`relay_address` has not been set in the config; hole punching will be unavailable until config reload.",
                ),
            );

            None
        };

        if opts.random_keys {
            use config::NodeMap;
            config.nodes = NodeMap::empty();
        }

        (
            Self {
                main_window,
                screen,
                modal,
                config,
                swarm_stream,
                swarm_sender: None,
                identity,
                theme,
                pane_logs,
                opts,
            },
            Task::none(),
        )
    }

    fn initial_setup() -> (Freecord, Task<Message>) {
        let opts = Opts::parse();

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

        let identity = if opts.random_keys {
            Identity::new_random_keys()
        } else {
            match Identity::load() {
                Ok(identity) => identity,
                Err(err) => match err {
                    identity::Error::NoPrivateKeyOnDisk => Identity::default(),
                    _ => panic!("{err}"),
                },
            }
        };

        let theme = Theme::default();

        let (main_window, open_main_window) = window::open(window::Settings {
            exit_on_close_request: false,
            ..window::settings()
        });

        let (freecord, new_task) = Self::new(
            window::Window::new(main_window),
            config,
            identity,
            theme,
            Vec::new(),
            opts.clone(),
        );

        let mut tasks = vec![Task::stream(log_stream).map(Message::Logging), new_task];

        if !opts.headless {
            tasks.push(open_main_window.discard());
        }

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
            Message::Identity(message) => {
                let Screen::Identity(identity) = &mut self.screen else {
                    return Task::none();
                };

                let (event_task, event) = identity.update(message, &self.identity);

                let task = if let Some(event) = event {
                    match event {
                        screen::identity::Event::GenerateSecretKey => {
                            let keys = identity::Keys::generate();
                            keys.save_private_key().unwrap_or_else(|e| {
                                log::error!("Failed to save private key to disk: {e}")
                            });
                            keys.save_public_key().unwrap_or_else(|e| {
                                log::error!("Failed to save public key to disk: {e}")
                            });
                            self.identity.keys = Some(keys);

                            Task::none()
                        }
                        screen::identity::Event::DeleteSecretKey => {
                            if let Some(keys) = &self.identity.keys {
                                keys.delete_keypair();
                            }

                            self.identity.keys = None;

                            Task::none()
                        }
                        screen::identity::Event::Exit => {
                            self.screen = Screen::Dashboard(screen::dashboard::Dashboard::new(
                                &self.main_window,
                            ));

                            Task::none()
                        }
                    }
                } else {
                    Task::none()
                };

                Task::batch(vec![task, event_task.map(Message::Identity)])
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
                        screen::dashboard::Event::SwitchToIdentity => {
                            self.screen = Screen::Identity(screen::identity::Identity::new());
                            Task::none()
                        }
                    }
                } else {
                    Task::none()
                };

                Task::batch(vec![task, event_task.map(Message::Dashboard)])
            }
            Message::Modal(message) => {
                let Some(modal) = &mut self.modal else {
                    return Task::none();
                };

                let (task, event) = modal.update(message);

                if let Some(event) = event {
                    match event {
                        modal::Event::Close => self.modal = None,
                    }
                }

                task.map(Message::Modal)
            }
            Message::Network(message) => match message {
                network::swarm::Message::NodeConnected(peer_id) => {
                    log::info!("[swarm] A node has connected to the swarm: {peer_id}");

                    Task::none()
                }
                network::swarm::Message::NodeDisconnected(peer_id) => {
                    log::info!("[swarm] A node has disconnected from the swarm: {peer_id}");

                    Task::none()
                }
                network::swarm::Message::SwarmCreated(sender) => {
                    log::info!("[swarm] Creation completed.");
                    self.swarm_sender = Some(sender);

                    Task::none()
                }
                network::swarm::Message::SwarmCreationError(error) => {
                    Self::push_modal_error(
                        &mut self.modal,
                        modal::Error::new(
                            "Networking error",
                            format!("Error during swarm creation: {error}"),
                        ),
                    );

                    self.swarm_stream = None;
                    self.swarm_sender = None;

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
            let screen = match &self.screen {
                Screen::Welcome(welcome) => welcome.view().map(Message::Welcome),
                Screen::Identity(identity) => identity.view(&self.identity).map(Message::Identity),
                Screen::Dashboard(dashboard) => dashboard
                    .view(&self.pane_logs, &self.config, environment::VERSION)
                    .map(Message::Dashboard),
            };

            match &self.modal {
                Some(modal) => {
                    widget::modal::modal(screen, modal.view().map(Message::Modal), || {
                        Message::Modal(modal::Message::Close)
                    })
                }
                None => screen,
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
        let mut subscriptions =
            vec![window::events().map(|(window, event)| Message::Window(window, event))];

        if let Some(swarm_stream) = self.swarm_stream.clone() {
            subscriptions.push(subscription::from_recipe(swarm_stream).map(Message::Network))
        }

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
    fn push_modal_error(modal: &mut Option<Modal>, error: modal::Error) {
        if let Some(Modal::Error(errors)) = modal {
            errors.push(error);
        } else {
            *modal = Some(Modal::Error(vec![error]));
        }
    }

    fn reload_config(&mut self, new_config_result: Result<Config, config::Error>) -> Task<Message> {
        let (freecord, task) = Self::new(
            self.main_window,
            new_config_result,
            self.identity.clone(),
            self.theme.clone(),
            self.pane_logs.clone(),
            self.opts.clone(),
        );

        *self = freecord;
        task
    }
}

#[derive(Clone, Parser)]
#[command(name = "freecord")]
#[command(version = environment::VERSION)]
#[command(about = "FOSS p2p chat app", long_about = None)]
struct Opts {
    #[arg(short, long, default_value_t = false)]
    random_keys: bool,
    #[arg(long, default_value_t = false)]
    headless: bool,
}
