use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use std::fmt::Debug;
use std::future::Future;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

pub trait Config: Debug + Default + Send + Sync + 'static {
    fn refresh_rate(&self) -> u64;
}

pub trait ConfigParser<T: Config>: Send + Sync {
    fn parse(&self, content: &str) -> Result<T>;
}

// TOML 解析器
pub struct TomlParser<T: Config + serde::de::DeserializeOwned> {
    _marker: PhantomData<T>,
}

impl<T: Config + serde::de::DeserializeOwned> Default for TomlParser<T> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T: Config + serde::de::DeserializeOwned> ConfigParser<T> for TomlParser<T> {
    fn parse(&self, content: &str) -> Result<T> {
        Ok(toml::from_str(content).context("Failed to parse TOML")?)
    }
}

// JSON 解析器
pub struct JsonParser<T: Config + serde::de::DeserializeOwned> {
    _marker: PhantomData<T>,
}

impl<T: Config + serde::de::DeserializeOwned> Default for JsonParser<T> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T: Config + serde::de::DeserializeOwned> ConfigParser<T> for JsonParser<T> {
    fn parse(&self, content: &str) -> Result<T> {
        Ok(serde_json::from_str(content).context("Failed to parse JSON")?)
    }
}

// YAML 解析器
pub struct YamlParser<T: Config + serde::de::DeserializeOwned> {
    _marker: PhantomData<T>,
}

impl<T: Config + serde::de::DeserializeOwned> Default for YamlParser<T> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T: Config + serde::de::DeserializeOwned> ConfigParser<T> for YamlParser<T> {
    fn parse(&self, content: &str) -> Result<T> {
        Ok(serde_yaml::from_str(content).context("Failed to parse YAML")?)
    }
}

// 配置管理器 - Parser 用 Arc 包裹
pub struct ConfigManager<T: Config, P: ConfigParser<T> = TomlParser<T>> {
    config: Arc<ArcSwap<T>>,
    parser: Arc<P>, // ✅ 使用 Arc 包裹 Parser
    watcher: Arc<Mutex<Option<JoinHandle<()>>>>,
    _marker: PhantomData<T>,
}

impl<T: Config, P: ConfigParser<T> + 'static> ConfigManager<T, P> {
    pub fn new(parser: P) -> Self {
        Self {
            config: Arc::new(ArcSwap::from_pointee(T::default())),
            parser: Arc::new(parser), // ✅ 放入 Arc
            watcher: Arc::new(Mutex::new(None)),
            _marker: PhantomData,
        }
    }

    pub fn stop(&self) {
        if let Some(handle) = self.watcher.lock().unwrap().take() {
            handle.abort();
        }
    }

    pub fn is_watching(&self) -> bool {
        self.watcher.lock().unwrap().is_some()
    }

    pub fn init(&self, path: PathBuf) -> Result<()> {
        self.start_watch(path, None)
    }

    pub fn init_call<D, F>(&self, path: PathBuf, call: Option<(D, F)>) -> Result<()>
    where
        D: Clone + Send + Sync + 'static,
        F: Fn(D) + Send + Sync + 'static,
    {
        let on_reload = call.map(|(data, callback)| {
            let callback = Arc::new(callback);
            OnReload::Sync(Box::new(move || {
                callback(data.clone());
            }))
        });
        self.start_watch(path, on_reload)
    }

    pub fn init_async_call<D, F, Fut>(&self, path: PathBuf, call: Option<(D, F)>) -> Result<()>
    where
        D: Clone + Send + Sync + 'static,
        F: Fn(D) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let on_reload = call.map(|(data, callback)| {
            let callback = Arc::new(callback);
            OnReload::Async(Box::new(move || {
                let data = data.clone();
                let callback = callback.clone();
                Box::pin(async move {
                    callback(data).await;
                }) as Pin<Box<dyn Future<Output = ()> + Send>>
            }))
        });
        self.start_watch(path, on_reload)
    }

    fn start_watch(&self, path: PathBuf, on_reload: Option<OnReload>) -> Result<()> {
        self.stop();
        self.load(&path)?;

        let mut curr_modified = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .context("Failed to get file metadata")?;

        let mut refresh_rate = self.get().refresh_rate();
        let config_ref = self.config.clone();
        let parser = self.parser.clone(); // ✅ 克隆 Arc（不会克隆内部数据）
        let path_clone = path.clone();

        let handle = tokio::spawn(async move {
            loop {
                if refresh_rate <= 5 {
                    refresh_rate = 5;
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(refresh_rate)).await;

                let result: Result<()> = async {
                    let modified = std::fs::metadata(&path_clone)
                        .and_then(|m| m.modified())
                        .context("Failed to get file metadata")?;

                    if curr_modified == modified {
                        return Ok(());
                    }

                    let content = std::fs::read_to_string(&path_clone)
                        .context("Failed to read config file")?;
                    let new_config = parser.parse(&content)?;
                    let new_config = Arc::new(new_config);

                    config_ref.store(new_config.clone());
                    refresh_rate = new_config.refresh_rate();
                    curr_modified = modified;

                    match &on_reload {
                        Some(OnReload::Sync(callback)) => callback(),
                        Some(OnReload::Async(callback)) => callback().await,
                        None => {}
                    }

                    log::info!("Config reloaded from {:?}", path_clone);
                    Ok(())
                }
                .await;

                if let Err(e) = result {
                    log::error!("Failed to reload config: {}", e);
                }
            }
        });
        *self.watcher.lock().unwrap() = Some(handle);

        Ok(())
    }

    pub fn load(&self, path: &PathBuf) -> Result<()> {
        let content = std::fs::read_to_string(path).context("Failed to read config file")?;

        let config = self.parser.parse(&content)?;
        self.config.store(Arc::new(config));
        Ok(())
    }

    pub fn get(&self) -> Arc<T> {
        self.config.load_full()
    }
}

pub type TomlConfigManager<T> = ConfigManager<T, TomlParser<T>>;
pub type JsonConfigManager<T> = ConfigManager<T, JsonParser<T>>;
pub type YamlConfigManager<T> = ConfigManager<T, YamlParser<T>>;

enum OnReload {
    Sync(Box<dyn Fn() + Send + Sync>),
    Async(Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>),
}
