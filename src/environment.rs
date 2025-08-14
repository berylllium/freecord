use std::env;
use std::path::PathBuf;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CONFIG_FILE_NAME: &str = "config.toml";
pub const XDG_DIR_NAME: &str = "freecord";

pub fn config_dir() -> PathBuf {
    portable_config_dir().unwrap_or_else(platform_specific_config_dir)
}

pub fn data_dir() -> PathBuf {
    portable_config_dir().unwrap_or_else(|| {
        dirs_next::data_dir()
            .expect("expected valid data dir")
            .join(XDG_DIR_NAME)
    })
}

pub fn cache_dir() -> PathBuf {
    dirs_next::cache_dir()
        .expect("expected valid data dir")
        .join(XDG_DIR_NAME)
}

fn portable_config_dir() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    let dir = exe.parent()?;

    dir.join(CONFIG_FILE_NAME)
        .is_file()
        .then(|| dir.to_path_buf())
}

fn platform_specific_config_dir() -> PathBuf {
    dirs_next::config_dir()
        .expect("expected valid config dir")
        .join(XDG_DIR_NAME)
}
