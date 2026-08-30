use crate::sql_cell::{SqlCell, SqlExecResult};
#[cfg(feature = "native")]
use anyhow::Context;
use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
pub trait SqlBackend: Send + Sync {
    async fn exec(
        &self,
        sql: &str,
        args: &[SqlCell],
        timeout: Duration,
    ) -> anyhow::Result<SqlExecResult>;

    #[cfg(feature = "native")]
    fn mysql_pool(&self) -> Option<&mysql_async::Pool> {
        None
    }

    async fn disconnect(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(feature = "native")]
pub struct MysqlBackend {
    pool: std::sync::Arc<mysql_async::Pool>,
}

#[cfg(feature = "native")]
fn url_query_usize(url: &str, key: &str) -> Option<usize> {
    let query = url.split_once('?')?.1;
    let query = query.split_once('#').map(|(q, _)| q).unwrap_or(query);
    let mut found = None;
    for pair in query.split('&') {
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };
        if k == key {
            if let Ok(n) = v.parse() {
                found = Some(n);
            }
        }
    }
    found
}

#[cfg(feature = "native")]
impl MysqlBackend {
    pub fn new(url: &str, max_idle_conns: usize, max_open_conns: usize) -> anyhow::Result<Self> {
        use mysql_async::{Opts, OptsBuilder, Pool, PoolConstraints};

        // mysql://localhost/db?pool_min=0&pool_max=151 — URL 有则用 URL（同名取最后一次），否则用参数
        let max_idle_conns = url_query_usize(url, "pool_min").unwrap_or(max_idle_conns);
        let max_open_conns = url_query_usize(url, "pool_max").unwrap_or(max_open_conns);
        let constraints =
            PoolConstraints::new(max_idle_conns, max_open_conns).ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid pool constraints: pool_min ({}) > pool_max ({}).",
                    max_idle_conns,
                    max_open_conns
                )
            })?;
        let opts = Opts::from_url(&crate::create::url_without_pool_bounds(url)).here()?;
        let pool_opts = opts.pool_opts().clone().with_constraints(constraints);
        let opts = OptsBuilder::from_opts(opts)
            .pool_opts(pool_opts)
            .prefer_socket(Some(false));
        Ok(Self {
            pool: std::sync::Arc::new(Pool::new(opts)),
        })
    }
}

#[cfg(feature = "native")]
#[async_trait]
impl SqlBackend for MysqlBackend {
    async fn exec(
        &self,
        sql: &str,
        args: &[SqlCell],
        timeout: Duration,
    ) -> anyhow::Result<SqlExecResult> {
        use mysql_async::prelude::Queryable;
        use mysql_async::{Row, Value};
        use rivetx_core::rivetx_string::RivetxString;

        let params: Vec<Value> = args.iter().cloned().map(Value::from).collect();
        let started = tokio::time::Instant::now();
        let mut conn = match tokio::time::timeout(timeout, self.pool.get_conn()).await {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => return Err(anyhow::anyhow!("pool.get_conn err:{}", e)),
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "sql exec timeout: {}ms (get_conn)",
                    timeout.as_millis()
                ));
            }
        };
        let remaining = timeout.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            let _ = conn.disconnect().await;
            return Err(anyhow::anyhow!(
                "sql exec timeout: {}ms (get_conn)",
                timeout.as_millis()
            ));
        }
        let fut = async {
            let mut result = conn.exec_iter(sql, params).await?;
            let affected = result.affected_rows();
            let last_insert_id = result.last_insert_id().unwrap_or(0);
            let cols: Vec<RivetxString> = result
                .columns_ref()
                .iter()
                .map(|c| RivetxString::from(c.name_str().to_string()))
                .collect();
            let mysql_rows: Vec<Row> = result.collect().await?;
            let mut rows = Vec::with_capacity(mysql_rows.len());
            for row in mysql_rows {
                let mut cells = Vec::with_capacity(cols.len());
                for i in 0..cols.len() {
                    let value: Value = row.get(i).unwrap_or(Value::NULL);
                    cells.push(SqlCell::from(value));
                }
                rows.push(cells);
            }
            Ok(SqlExecResult {
                cols,
                rows,
                affected,
                last_insert_id,
            })
        };

        match tokio::time::timeout(remaining, fut).await {
            Ok(v) => v,
            Err(_) => {
                // 查询做到一半被取消时协议序号已经乱了；必须 disconnect，
                // 否则 Drop 把半截连接还回池，下次 get_conn 会 packet out of order。
                let _ = conn.disconnect().await;
                Err(anyhow::anyhow!(
                    "sql exec timeout: {}ms",
                    timeout.as_millis()
                ))
            }
        }
    }

    fn mysql_pool(&self) -> Option<&mysql_async::Pool> {
        Some(&self.pool)
    }

    async fn disconnect(&self) -> anyhow::Result<()> {
        (*self.pool)
            .clone()
            .disconnect()
            .await
            .map_err(|e| anyhow::anyhow!("mysql pool disconnect err:{}", e))
    }
}
