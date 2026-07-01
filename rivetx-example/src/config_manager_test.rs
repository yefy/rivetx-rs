use anyhow::Context;
use arc_swap::ArcSwap;
use log::info;
use once_cell::sync::Lazy;
use rivetx_core::config_manager::{Config, ConfigManager, TomlConfigManager, TomlParser};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub refresh_rate: u64,
    pub app_name: String,
    pub max_connections: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            refresh_rate: 5,
            app_name: String::new(),
            max_connections: 0,
        }
    }
}

impl Config for AppConfig {
    fn refresh_rate(&self) -> u64 {
        self.refresh_rate
    }
}

static CONFIGX: Lazy<ArcSwap<ConfigManager<AppConfig>>> =
    Lazy::new(|| ArcSwap::from_pointee(TomlConfigManager::<AppConfig>::new(TomlParser::default())));

pub fn config() -> Arc<AppConfig> {
    CONFIGX.load_full().get()
}

pub async fn config_manager_tests() -> anyhow::Result<()> {
    let config_path = PathBuf::from("./conf/app.toml");

    let manager = TomlConfigManager::<AppConfig>::new(TomlParser::default());
    manager.load(&config_path).here()?;

    let cfg = manager.get();
    info!(
        "loaded config: app_name={}, max_connections={}, refresh_rate={}",
        cfg.app_name, cfg.max_connections, cfg.refresh_rate
    );

    manager.init(config_path.clone()).here()?;

    CONFIGX.store(Arc::new(manager));

    let updated = r#"refresh_rate = 5
app_name = "rivetx-example-reloaded"
max_connections = 200
"#;
    std::fs::write(&config_path, updated).context("Failed to write config file")?;

    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

    let cfg = config();
    info!(
        "reloaded config: app_name={}, max_connections={}, refresh_rate={}",
        cfg.app_name, cfg.max_connections, cfg.refresh_rate
    );

    assert_eq!(cfg.app_name, "rivetx-example-reloaded");
    assert_eq!(cfg.max_connections, 200);

    let original = r#"refresh_rate = 5
app_name = "rivetx-example"
max_connections = 100
"#;
    std::fs::write(&config_path, original).context("Failed to restore config file")?;

    Ok(())
}
