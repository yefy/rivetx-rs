use crate::backend::SqlBackend;
use crate::sql_cell::{SqlCell, SqlExecResult};
#[cfg(feature = "native")]
use anyhow::Context;
use rivetx_core::rivetx_string::RivetxString;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct RivetxSql {
    name: RivetxString,
    backend: Arc<dyn SqlBackend>,
}

impl RivetxSql {
    #[cfg(feature = "native")]
    pub fn new(url: &str, max_idle_conns: usize, max_open_conns: usize) -> anyhow::Result<Self> {
        let backend = Arc::new(crate::backend::MysqlBackend::new(
            url,
            max_idle_conns,
            max_open_conns,
        )?);
        Ok(Self {
            name: RivetxString::from("default"),
            backend,
        })
    }

    pub fn from_backend(name: impl Into<RivetxString>, backend: Arc<dyn SqlBackend>) -> Self {
        Self {
            name: name.into(),
            backend,
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub async fn exec(
        &self,
        sql: &str,
        args: &[SqlCell],
        timeout: Duration,
    ) -> anyhow::Result<SqlExecResult> {
        self.backend.exec(sql, args, timeout).await
    }

    pub async fn disconnect(&self) -> anyhow::Result<()> {
        self.backend.disconnect().await
    }
}

#[cfg(feature = "native")]
impl RivetxSql {
    pub async fn get_conn(&self) -> anyhow::Result<mysql_async::Conn> {
        self.conn().await
    }

    pub async fn conn(&self) -> anyhow::Result<mysql_async::Conn> {
        let pool = self
            .backend
            .mysql_pool()
            .ok_or_else(|| anyhow::anyhow!("RivetxSql backend is not mysql"))?;
        pool.get_conn()
            .await
            .map_err(|e| anyhow::anyhow!("pool.get_conn err:{}", e))
            .here()
    }
}
