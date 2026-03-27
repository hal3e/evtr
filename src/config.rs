mod file;
mod keymap;
mod path;
mod types;
mod validate;

use std::fs;
use std::path::Path;
use std::sync::{LazyLock, RwLock};

use crate::config::file::ConfigFile;
use crate::config::path::resolved_read_path;
use crate::error::{Error, ErrorArea, Result};

pub(crate) use self::keymap::KeyBinding;
pub(crate) use self::types::{
    Config, KeymapConfig, LayoutConfig, MonitorConfig, SelectorConfig, SelectorLayoutConfig,
    SortOrder, StartupFocus, ThemeConfig,
};

static RUNTIME_CONFIG: LazyLock<RwLock<Config>> = LazyLock::new(|| RwLock::new(Config::default()));

pub(crate) fn load(explicit_path: Option<&std::path::Path>) -> Result<Config> {
    let Some(path) = resolved_read_path(explicit_path)? else {
        return Ok(Config::default());
    };

    let content = fs::read_to_string(&path)
        .map_err(|err| ErrorArea::Config.io(format!("read {}", path.display()), err))?;
    let file = toml::from_str::<ConfigFile>(&content)
        .map_err(|err| Error::config(format!("invalid config {}: {}", path.display(), err)))?;

    Config::try_from(file)
}

pub(crate) fn resolved_write_path(
    explicit_path: Option<&std::path::Path>,
) -> Result<std::path::PathBuf> {
    path::resolved_write_path(explicit_path)
}

pub(crate) fn render_default_config() -> Result<String> {
    ConfigFile::render_default()
}

pub(crate) fn write_default_config(path: &Path) -> Result<()> {
    if path.exists() {
        return Err(Error::config(format!(
            "config file already exists: {}",
            path.display()
        )));
    }

    let Some(parent) = path.parent() else {
        return Err(Error::config(format!(
            "config path has no parent directory: {}",
            path.display()
        )));
    };

    fs::create_dir_all(parent)
        .map_err(|err| ErrorArea::Config.io(format!("create {}", parent.display()), err))?;
    fs::write(path, ConfigFile::render_default()?)
        .map_err(|err| ErrorArea::Config.io(format!("write {}", path.display()), err))?;
    Ok(())
}

pub(crate) fn install_runtime(config: Config) {
    *RUNTIME_CONFIG
        .write()
        .expect("runtime config lock poisoned") = config;
}

pub(crate) fn app() -> Config {
    RUNTIME_CONFIG
        .read()
        .expect("runtime config lock poisoned")
        .clone()
}

pub(crate) fn selector() -> SelectorConfig {
    app().selector
}

pub(crate) fn monitor() -> MonitorConfig {
    app().monitor
}

pub(crate) fn theme() -> ThemeConfig {
    app().theme
}

pub(crate) fn layout() -> LayoutConfig {
    app().layout
}

pub(crate) fn keys() -> KeymapConfig {
    app().keys
}
