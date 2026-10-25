// ------------------------------------------------------------
// Module: config
// Purpose: Load the host settings from config.toml, then let
//          environment variables override them. A missing GM
//          key is generated so a fresh checkout can boot.
// ------------------------------------------------------------

use std::path::PathBuf;

use serde::Deserialize;

/// Settings for one hosted table.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Socket address the server binds, for example `0.0.0.0:8080`.
    pub bind: String,
    /// Secret that promotes a connection to the GM role.
    pub gm_key: String,
    /// Shared phrase players type in the lobby.
    pub table_passphrase: String,
    /// Directory for the save file, map images, and token portraits.
    pub data_dir: String,
    /// Built client to serve. Missing in dev, when Vite serves the UI.
    pub static_dir: String,
    /// Asset packs. Each pack is a folder under `packs/`.
    pub assets_dir: String,
    /// Game systems. Each system is a folder with one system.json.
    pub systems_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:8080".into(),
            gm_key: String::new(),
            table_passphrase: "table".into(),
            data_dir: "data".into(),
            static_dir: "client/dist".into(),
            assets_dir: "assets".into(),
            systems_dir: "systems".into(),
        }
    }
}

impl Config {
    /// Read `RPG_CONFIG`, then `./config.toml`, then `../config.toml`.
    pub fn load() -> Self {
        let mut config = Self::default();
        let path = std::env::var("RPG_CONFIG")
            .ok()
            .map(PathBuf::from)
            .or_else(|| existing("config.toml"))
            .or_else(|| existing("../config.toml"));

        if let Some(path) = path {
            match std::fs::read_to_string(&path) {
                Ok(text) => match toml::from_str::<Config>(&text) {
                    Ok(parsed) => {
                        tracing::info!("loaded config from {}", path.display());
                        config = parsed;
                    }
                    Err(err) => {
                        tracing::warn!("could not parse {}: {err}", path.display());
                    }
                },
                Err(err) => tracing::warn!("could not read {}: {err}", path.display()),
            }
        }

        if let Ok(value) = std::env::var("RPG_BIND") {
            if !value.is_empty() {
                config.bind = value;
            }
        }
        if let Ok(value) = std::env::var("RPG_GM_KEY") {
            if !value.is_empty() {
                config.gm_key = value;
            }
        }
        if let Ok(value) = std::env::var("RPG_PASSPHRASE") {
            if !value.is_empty() {
                config.table_passphrase = value;
            }
        }
        if let Ok(value) = std::env::var("RPG_DATA_DIR") {
            if !value.is_empty() {
                config.data_dir = value;
            }
        }
        if let Ok(value) = std::env::var("RPG_STATIC_DIR") {
            if !value.is_empty() {
                config.static_dir = value;
            }
        }
        if let Ok(value) = std::env::var("RPG_ASSETS_DIR") {
            if !value.is_empty() {
                config.assets_dir = value;
            }
        }
        if let Ok(value) = std::env::var("RPG_SYSTEMS_DIR") {
            if !value.is_empty() {
                config.systems_dir = value;
            }
        }

        if config.gm_key.trim().is_empty() {
            config.gm_key = remembered_key(&config.data_dir);
        }
        config.gm_key = config.gm_key.trim().to_string();
        config.table_passphrase = config.table_passphrase.trim().to_string();
        config
    }
}

fn existing(path: &str) -> Option<PathBuf> {
    let path = PathBuf::from(path);
    path.is_file().then_some(path)
}

/// An empty config keeps one key in the data directory, so a restart
/// does not print a new key and lock the GM out of the old one.
fn remembered_key(data_dir: &str) -> String {
    let dir = PathBuf::from(data_dir);
    let path = dir.join("host.key");
    if let Ok(text) = std::fs::read_to_string(&path) {
        let text = text.trim().to_string();
        if !text.is_empty() {
            return text;
        }
    }
    let key = generate_key();
    if let Err(err) = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, &key)) {
        tracing::warn!("could not remember the GM key: {err}");
    }
    key
}

fn generate_key() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    (0..6)
        .map(|_| {
            let index = rand::random_range(0..ALPHABET.len());
            ALPHABET[index] as char
        })
        .collect()
}
