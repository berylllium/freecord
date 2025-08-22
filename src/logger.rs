use chrono::{DateTime, Utc};
use log::Log;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::ReceiverStream;

use std::{
    cmp::Ordering,
    env, fmt, fs, io,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use crate::{
    config::{self, logs::LevelFilter},
    environment,
};

pub fn setup(
    is_debug: bool,
    config: config::logs::Logs,
) -> Result<ReceiverStream<Vec<Record>>, Error> {
    let env_rust_log = env::var("RUST_LOG")
        .ok()
        .as_deref()
        .map(str::parse::<log::Level>)
        .transpose()?;

    let file_level_filter = env_rust_log.map_or(log::LevelFilter::from(config.file_level), |env| {
        env.to_level_filter()
    });

    let panel_level_filter =
        env_rust_log.map_or(log::LevelFilter::Trace, |env| env.to_level_filter());

    let mut io_sink = fern::Dispatch::new().format(|out, message, record| {
        out.finish(format_args!(
            "{}:{} -- {}",
            chrono::Local::now().format("%H:%M:%S%.3f"),
            record.level(),
            message
        ));
    });

    if is_debug {
        io_sink = io_sink.chain(std::io::stdout());
    } else {
        let log_file = file()?;

        io_sink = io_sink.chain(log_file);
    }

    io_sink = io_sink
        .level(log::LevelFilter::Off)
        .level_for("panic", log::LevelFilter::Error)
        .level_for("iced_wgpu", log::LevelFilter::Info)
        .level_for("freecord", file_level_filter);

    let (channel_sink, receiver) = panel_logger();

    let panel_sink = fern::Dispatch::new()
        .chain(channel_sink)
        .level(log::LevelFilter::Off)
        .level_for("panic", log::LevelFilter::Error)
        .level_for("iced_wgpu", log::LevelFilter::Info)
        .level_for("freecord", panel_level_filter);

    fern::Dispatch::new()
        .chain(io_sink)
        .chain(panel_sink)
        .apply()?;

    Ok(receiver)
}

fn panel_logger() -> (Box<dyn Log>, ReceiverStream<Vec<Record>>) {
    struct Sink {
        sender: mpsc::Sender<Record>,
    }

    impl Log for Sink {
        fn enabled(&self, _metadata: &log::Metadata) -> bool {
            true
        }

        fn log(&self, record: &log::Record) {
            let _ = self.sender.send(Record {
                timestamp: Utc::now(),
                level: record.level().into(),
                message: format!("{}", record.args()),
            });
        }

        fn flush(&self) {}
    }

    let (log_sender, log_receiver) = mpsc::channel();
    let (async_sender, async_receiver) = tokio::sync::mpsc::channel(1);

    thread::spawn(move || {
        const BATCH_SIZE: usize = 25;
        const BATCH_TIMEOUT: Duration = Duration::from_millis(250);

        let mut batch = Vec::with_capacity(BATCH_SIZE);
        let mut timeout = Instant::now();

        loop {
            if let Ok(log) = log_receiver.recv_timeout(BATCH_TIMEOUT) {
                batch.push(log);
            }

            if batch.len() >= BATCH_SIZE
                || (!batch.is_empty() && timeout.elapsed() >= BATCH_TIMEOUT)
            {
                timeout = Instant::now();

                let _ = async_sender.blocking_send(std::mem::replace(
                    &mut batch,
                    Vec::with_capacity(BATCH_SIZE),
                ));
            }
        }
    });

    (
        Box::new(Sink { sender: log_sender }),
        ReceiverStream::new(async_receiver),
    )
}

pub fn file() -> Result<fs::File, Error> {
    let path = path()?;

    Ok(fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(false)
        .truncate(true)
        .open(path)?)
}

fn path() -> Result<PathBuf, Error> {
    let parent = environment::data_dir();

    if !parent.exists() {
        fs::create_dir_all(&parent)?;
    }

    Ok(parent.join("freecord.log"))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Record {
    pub timestamp: DateTime<Utc>,
    pub level: Level,
    pub message: String,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Serialize, Deserialize)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Level::Error => write!(f, "ERROR"),
            Level::Warn => write!(f, "WARN"),
            Level::Info => write!(f, "INFO"),
            Level::Debug => write!(f, "DEBUG"),
            Level::Trace => write!(f, "TRACE"),
        }
    }
}
impl From<log::Level> for Level {
    fn from(level: log::Level) -> Self {
        match level {
            log::Level::Error => Level::Error,
            log::Level::Warn => Level::Warn,
            log::Level::Info => Level::Info,
            log::Level::Debug => Level::Debug,
            log::Level::Trace => Level::Trace,
        }
    }
}

impl std::cmp::PartialOrd<LevelFilter> for Level {
    fn partial_cmp(&self, other: &LevelFilter) -> Option<Ordering> {
        Some(match self {
            Level::Error => match other {
                LevelFilter::Off => Ordering::Greater,
                LevelFilter::Error => Ordering::Equal,
                LevelFilter::Warn | LevelFilter::Info | LevelFilter::Debug | LevelFilter::Trace => {
                    Ordering::Less
                }
            },
            Level::Warn => match other {
                LevelFilter::Off | LevelFilter::Error => Ordering::Greater,
                LevelFilter::Warn => Ordering::Equal,
                LevelFilter::Info | LevelFilter::Debug | LevelFilter::Trace => Ordering::Less,
            },
            Level::Info => match other {
                LevelFilter::Off | LevelFilter::Error | LevelFilter::Warn => Ordering::Greater,
                LevelFilter::Info => Ordering::Equal,
                LevelFilter::Debug | LevelFilter::Trace => Ordering::Less,
            },
            Level::Debug => match other {
                LevelFilter::Off | LevelFilter::Error | LevelFilter::Warn | LevelFilter::Info => {
                    Ordering::Greater
                }
                LevelFilter::Debug => Ordering::Equal,
                LevelFilter::Trace => Ordering::Less,
            },
            Level::Trace => match other {
                LevelFilter::Off
                | LevelFilter::Error
                | LevelFilter::Warn
                | LevelFilter::Info
                | LevelFilter::Debug => Ordering::Greater,
                LevelFilter::Trace => Ordering::Equal,
            },
        })
    }
}

impl std::cmp::PartialEq<LevelFilter> for Level {
    fn eq(&self, other: &LevelFilter) -> bool {
        match self {
            Level::Error => matches!(other, LevelFilter::Error),
            Level::Warn => matches!(other, LevelFilter::Warn),
            Level::Info => matches!(other, LevelFilter::Info),
            Level::Debug => matches!(other, LevelFilter::Debug),
            Level::Trace => matches!(other, LevelFilter::Trace),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    SetLog(#[from] log::SetLoggerError),
    #[error(transparent)]
    ParseLevel(#[from] log::ParseLevelError),
}
