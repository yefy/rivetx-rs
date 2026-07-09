use anyhow::Context;
use arc_swap::ArcSwap;
use log::info;
use once_cell::sync::Lazy;
use rivetx_core::config_manager::{Config, ConfigManager, TomlConfigManager, TomlParser};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

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

    init_call_tests().await?;
    init_async_call_tests().await?;
    stop_tests().await?;

    CONFIGX.load_full().stop();
    assert!(!CONFIGX.load_full().is_watching());

    Ok(())
}

#[derive(Clone)]
struct CallbackData {
    tag: String,
}

async fn init_call_tests() -> anyhow::Result<()> {
    let config_path = PathBuf::from("./conf/app_callback.toml");

    let initial = r#"refresh_rate = 5
app_name = "rivetx-example-callback"
max_connections = 100
"#;
    std::fs::write(&config_path, initial).context("Failed to write callback config file")?;

    let manager = TomlConfigManager::<AppConfig>::new(TomlParser::default());

    let called = Arc::new(AtomicBool::new(false));
    let received_tag = Arc::new(Mutex::new(String::new()));

    let called_clone = called.clone();
    let received_tag_clone = received_tag.clone();
    let expected_tag = "init_call_test".to_string();

    manager
        .init_call(
            config_path.clone(),
            Some((
                CallbackData {
                    tag: expected_tag.clone(),
                },
                move |d: CallbackData| {
                    called_clone.store(true, Ordering::SeqCst);
                    *received_tag_clone.lock().unwrap() = d.tag;
                },
            )),
        )
        .here()?;

    let updated = r#"refresh_rate = 5
app_name = "rivetx-example-callback-reloaded"
max_connections = 300
"#;
    std::fs::write(&config_path, updated).context("Failed to write callback config file")?;

    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

    assert!(called.load(Ordering::SeqCst), "init_call callback was not invoked");
    assert_eq!(*received_tag.lock().unwrap(), expected_tag);

    let cfg = manager.get();
    info!(
        "init_call reloaded config: app_name={}, max_connections={}, refresh_rate={}",
        cfg.app_name, cfg.max_connections, cfg.refresh_rate
    );

    assert_eq!(cfg.app_name, "rivetx-example-callback-reloaded");
    assert_eq!(cfg.max_connections, 300);

    manager.stop();
    assert!(!manager.is_watching());

    std::fs::remove_file(&config_path).ok();

    Ok(())
}

async fn init_async_call_tests() -> anyhow::Result<()> {
    let config_path = PathBuf::from("./conf/app_async_callback.toml");

    let initial = r#"refresh_rate = 5
app_name = "rivetx-example-async-callback"
max_connections = 100
"#;
    std::fs::write(&config_path, initial).context("Failed to write async callback config file")?;

    let manager = TomlConfigManager::<AppConfig>::new(TomlParser::default());

    let called = Arc::new(AtomicBool::new(false));
    let received_tag = Arc::new(Mutex::new(String::new()));

    let called_clone = called.clone();
    let received_tag_clone = received_tag.clone();
    let expected_tag = "init_async_call_test".to_string();

    manager
        .init_async_call(
            config_path.clone(),
            Some((
                CallbackData {
                    tag: expected_tag.clone(),
                },
                move |d: CallbackData| {
                    let called_clone = called_clone.clone();
                    let received_tag_clone = received_tag_clone.clone();
                    async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                        called_clone.store(true, Ordering::SeqCst);
                        *received_tag_clone.lock().unwrap() = d.tag;
                    }
                },
            )),
        )
        .here()?;

    let updated = r#"refresh_rate = 5
app_name = "rivetx-example-async-callback-reloaded"
max_connections = 400
"#;
    std::fs::write(&config_path, updated).context("Failed to write async callback config file")?;

    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

    assert!(
        called.load(Ordering::SeqCst),
        "init_async_call callback was not invoked"
    );
    assert_eq!(*received_tag.lock().unwrap(), expected_tag);

    let cfg = manager.get();
    info!(
        "init_async_call reloaded config: app_name={}, max_connections={}, refresh_rate={}",
        cfg.app_name, cfg.max_connections, cfg.refresh_rate
    );

    assert_eq!(cfg.app_name, "rivetx-example-async-callback-reloaded");
    assert_eq!(cfg.max_connections, 400);

    manager.stop();
    assert!(!manager.is_watching());

    std::fs::remove_file(&config_path).ok();

    Ok(())
}

async fn stop_tests() -> anyhow::Result<()> {
    let config_path = PathBuf::from("./conf/app_stop.toml");

    let initial = r#"refresh_rate = 5
app_name = "rivetx-example-stop-v1"
max_connections = 100
"#;
    std::fs::write(&config_path, initial).context("Failed to write stop config file")?;

    let manager = TomlConfigManager::<AppConfig>::new(TomlParser::default());
    assert!(!manager.is_watching());

    manager.init(config_path.clone()).here()?;
    assert!(manager.is_watching());

    let updated_v1 = r#"refresh_rate = 5
app_name = "rivetx-example-stop-v2"
max_connections = 200
"#;
    std::fs::write(&config_path, updated_v1).context("Failed to write stop config file")?;

    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

    let cfg = manager.get();
    info!(
        "stop test after first reload: app_name={}, max_connections={}",
        cfg.app_name, cfg.max_connections
    );
    assert_eq!(cfg.app_name, "rivetx-example-stop-v2");
    assert_eq!(cfg.max_connections, 200);

    manager.stop();
    assert!(!manager.is_watching());

    let reload_count = Arc::new(AtomicU32::new(0));
    let reload_count_clone = reload_count.clone();
    manager
        .init_call(
            config_path.clone(),
            Some((
                CallbackData {
                    tag: "stop_test".to_string(),
                },
                move |_d: CallbackData| {
                    reload_count_clone.fetch_add(1, Ordering::SeqCst);
                },
            )),
        )
        .here()?;
    assert!(manager.is_watching());

    manager.stop();
    assert!(!manager.is_watching());

    let updated_v3 = r#"refresh_rate = 5
app_name = "rivetx-example-stop-v3"
max_connections = 300
"#;
    std::fs::write(&config_path, updated_v3).context("Failed to write stop config file")?;

    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

    let cfg = manager.get();
    info!(
        "stop test after stop: app_name={}, max_connections={}, reload_count={}",
        cfg.app_name,
        cfg.max_connections,
        reload_count.load(Ordering::SeqCst)
    );

    assert_eq!(cfg.app_name, "rivetx-example-stop-v2");
    assert_eq!(cfg.max_connections, 200);
    assert_eq!(reload_count.load(Ordering::SeqCst), 0);

    std::fs::remove_file(&config_path).ok();

    Ok(())
}
