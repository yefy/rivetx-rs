use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::Arc;

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
    _marker: PhantomData<T>,
}

impl<T: Config, P: ConfigParser<T> + 'static> ConfigManager<T, P> {
    pub fn new(parser: P) -> Self {
        Self {
            config: Arc::new(ArcSwap::from_pointee(T::default())),
            parser: Arc::new(parser), // ✅ 放入 Arc
            _marker: PhantomData,
        }
    }

    pub fn init(&self, path: PathBuf) -> Result<()> {
        self.load(&path)?;

        let mut curr_modified = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .context("Failed to get file metadata")?;

        let mut refresh_rate = self.get().refresh_rate();
        let config_ref = self.config.clone();
        let parser = self.parser.clone(); // ✅ 克隆 Arc（不会克隆内部数据）
        let path_clone = path.clone();

        tokio::spawn(async move {
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

                    config_ref.store(Arc::new(new_config));
                    refresh_rate = config_ref.load_full().refresh_rate();
                    curr_modified = modified;

                    log::info!("Config reloaded from {:?}", path_clone);
                    Ok(())
                }
                .await;

                if let Err(e) = result {
                    log::error!("Failed to reload config: {}", e);
                }
            }
        });

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
