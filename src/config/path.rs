use std::{
    env,
    path::{Path, PathBuf},
};

use crate::error::{Error, Result};

const APP_NAME: &str = "evtr";
const CONFIG_FILE_NAME: &str = "config.toml";

pub(crate) fn resolved_read_path(explicit_path: Option<&Path>) -> Result<Option<PathBuf>> {
    if let Some(path) = explicit_path {
        if !path.exists() {
            return Err(Error::config(format!(
                "config file does not exist: {}",
                path.display()
            )));
        }
        return Ok(Some(path.to_path_buf()));
    }

    if let Some(path) = xdg_config_path() {
        if path.is_file() {
            return Ok(Some(path));
        }
    }

    let fallback = dot_config_path()?;
    if fallback.is_file() {
        return Ok(Some(fallback));
    }

    Ok(None)
}

pub(crate) fn resolved_write_path(explicit_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit_path {
        return Ok(path.to_path_buf());
    }

    if let Some(root) = xdg_config_root() {
        if root.exists() {
            return Ok(root.join(APP_NAME).join(CONFIG_FILE_NAME));
        }
    }

    dot_config_path()
}

fn xdg_config_path() -> Option<PathBuf> {
    xdg_config_root().map(|root| root.join(APP_NAME).join(CONFIG_FILE_NAME))
}

fn xdg_config_root() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

fn dot_config_path() -> Result<PathBuf> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| Error::config("unable to resolve default config path: HOME is not set"))?;
    Ok(home.join(".config").join(APP_NAME).join(CONFIG_FILE_NAME))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::{LazyLock, Mutex},
    };

    use super::{resolved_read_path, resolved_write_path};

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn set_env(name: &str, value: Option<&str>) -> Option<String> {
        let old = std::env::var(name).ok();
        match value {
            Some(value) => unsafe { std::env::set_var(name, value) },
            None => unsafe { std::env::remove_var(name) },
        }
        old
    }

    fn restore_env(name: &str, old: Option<String>) {
        match old {
            Some(value) => unsafe { std::env::set_var(name, value) },
            None => unsafe { std::env::remove_var(name) },
        }
    }

    #[test]
    fn resolved_write_path_prefers_existing_xdg_root() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = tempfile_dir("config-xdg");
        let home = tempfile_dir("config-home");
        let old_home = set_env("HOME", Some(home.to_str().unwrap()));
        let old_xdg = set_env("XDG_CONFIG_HOME", Some(tmp.to_str().unwrap()));

        let path = resolved_write_path(None).unwrap();

        restore_env("HOME", old_home);
        restore_env("XDG_CONFIG_HOME", old_xdg);

        assert!(path.starts_with(tmp));
    }

    #[test]
    fn resolved_write_path_falls_back_to_home_config_when_xdg_root_is_missing() {
        let _guard = ENV_LOCK.lock().unwrap();
        let home = tempfile_dir("config-home");
        let missing = home.join("missing-xdg");
        let old_home = set_env("HOME", Some(home.to_str().unwrap()));
        let old_xdg = set_env("XDG_CONFIG_HOME", Some(missing.to_str().unwrap()));

        let path = resolved_write_path(None).unwrap();

        restore_env("HOME", old_home);
        restore_env("XDG_CONFIG_HOME", old_xdg);

        assert_eq!(path, home.join(".config").join("evtr").join("config.toml"));
    }

    #[test]
    fn resolved_read_path_uses_explicit_path() {
        let tmp = tempfile_dir("config-read");
        let config = tmp.join("custom.toml");
        fs::write(&config, "").unwrap();

        assert_eq!(resolved_read_path(Some(&config)).unwrap(), Some(config));
    }

    fn tempfile_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("evtr-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
