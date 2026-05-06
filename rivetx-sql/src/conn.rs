use mysql_async::{Conn, Opts, OptsBuilder, Pool, PoolConstraints, PoolOpts};
use std::sync::Arc;

#[derive(Clone)]
pub struct RivetxSql {
    pool: Arc<Pool>,
}

impl RivetxSql {
    pub fn new(url: &str, max_idle_conns: usize, max_open_conns: usize) -> anyhow::Result<Self> {
        let opts = Opts::from_url(url)?;
        let constraints = PoolConstraints::new(max_idle_conns, max_open_conns)
            .ok_or(anyhow::anyhow!("PoolConstraints::new"))?;
        let pool_opts = PoolOpts::new().with_constraints(constraints);

        let opts = OptsBuilder::from_opts(opts).pool_opts(pool_opts);

        let pool = Pool::new(opts);

        Ok(Self {
            pool: Arc::new(pool),
        })
    }
}

impl RivetxSql {
    pub async fn get_conn(&self) -> anyhow::Result<Conn> {
        return self.conn().await;
    }
    pub async fn conn(&self) -> anyhow::Result<Conn> {
        self.pool
            .get_conn()
            .await
            .map_err(|e| anyhow::anyhow!("pool.get_conn err:{}", e))
    }
}
