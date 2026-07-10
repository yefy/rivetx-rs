use crate::conn::RivetxSql;
use crate::util::{BATCH_SIZE, TIMEOUT};
use crate::{FromSqlRow, ToSqlValues};
use anyhow::Context;
use anyhow::{anyhow, Result};
use mysql_async::{prelude::*, Value};
use rivetx_core::rivetx_string::RivetxString;
use std::time::{Duration, Instant};
use rivetx_core::rivetx_str::RivetxStr;

#[derive(Debug, Clone, Default)]
pub struct UpdateResult {
    pub total_affected: u64,
    pub last_insert_id: u64,
}

pub async fn update_raw(
    rivetx_sql: &RivetxSql,
    table: &RivetxStr<'_>,
    cols: &Vec<RivetxString>,
    mut vals: Vec<Vec<Value>>,
    join_on: &Vec<RivetxString>,
    set_expr: &Vec<RivetxString>,
    mut max_per_batch: usize,
    mut execution_timeout: Duration,
) -> Result<UpdateResult> {
    if vals.is_empty() || cols.is_empty() || join_on.is_empty() || set_expr.is_empty() {
        return Err(anyhow!("vals, cols, join_on, or set_expr is empty"));
    }

    for (i, v_row) in vals.iter().enumerate() {
        if v_row.len() != cols.len() {
            return Err(anyhow!(
                "vals[{}] length {} does not match cols length {}",
                i,
                v_row.len(),
                cols.len()
            ));
        }
    }

    if max_per_batch == 0 {
        max_per_batch = BATCH_SIZE;
    }

    if execution_timeout == Duration::from_secs(0) {
        execution_timeout = TIMEOUT;
    }

    let start_time = Instant::now();
    let mut total_affected = 0u64;
    let mut last_insert_id = 0u64;

    while !vals.is_empty() {
        let chunk: Vec<_> = vals.drain(..max_per_batch.min(vals.len())).collect();
        let mut rows_sql = Vec::with_capacity(chunk.len() * 4);
        let mut args = Vec::with_capacity(chunk.len() * cols.len());

        for v in chunk {
            let placeholders = vec!["?"; v.len()].join(",");
            rows_sql.push(format!("ROW({})", placeholders));
            args.extend(v);
        }

        let on_conditions: Vec<String> = join_on
            .iter()
            .map(|c| format!("u.{} = v.{}", c, c))
            .collect();

        let query = format!(
            "UPDATE {} AS u JOIN (VALUES {}) AS v({}) ON {} SET {}",
            table,
            rows_sql.join(", "),
            cols.join(", "),
            on_conditions.join(" AND "),
            set_expr.join(", ")
        );

        let exec_start = Instant::now();
        let mut conn = rivetx_sql.get_conn().await.here()?;
        let res = tokio::time::timeout(execution_timeout, conn.exec_iter(&query, &args))
            .await
            .map_err(|e| anyhow::anyhow!( "batch_start: {}, allTime: {}ms, execTime: {}ms, totalAffected: {}, rowsAffected: {}, lastInsertId: {}, args:{:?}, err:{}",
            total_affected - 0,
            start_time.elapsed().as_millis(),
            exec_start.elapsed().as_millis(),
            total_affected,
            0,
            last_insert_id,
            args, e))?
            .map_err(|e| anyhow::anyhow!( "batch_start: {}, allTime: {}ms, execTime: {}ms, totalAffected: {}, rowsAffected: {}, lastInsertId: {}, args:{:?}, err:{}",
            total_affected - 0,
            start_time.elapsed().as_millis(),
            exec_start.elapsed().as_millis(),
            total_affected,
            0,
            last_insert_id,
            args, e))?;

        let affected = res.affected_rows();
        let last_id = res.last_insert_id().unwrap_or(0);

        total_affected += affected;
        if affected > 0 {
            last_insert_id = last_id + affected - 1;
        }

        log::debug!(
            "batch_start: {}, allTime: {}ms, execTime: {}ms, totalAffected: {}, rowsAffected: {}, lastInsertId: {}, args:{:?}",
            total_affected - affected,
            start_time.elapsed().as_millis(),
            exec_start.elapsed().as_millis(),
            total_affected,
            affected,
            last_insert_id,
            args
        );
    }

    Ok(UpdateResult {
        total_affected,
        last_insert_id,
    })
}

pub async fn update<T>(
    rivetx_sql: &RivetxSql,
    table: &RivetxString,
    vals: &Vec<T>,
    join_on: &Vec<RivetxString>,
    set_expr: &Vec<RivetxString>,
    max_per_batch: usize,
    timeout: Duration,
) -> Result<UpdateResult>
where
    T: FromSqlRow + ToSqlValues + Send + Sync, // Assume the Entity trait provides metadata
{
    if vals.is_empty() || join_on.is_empty() || set_expr.is_empty() {
        return Err(anyhow!("vals, join_on, or set_expr is empty"));
    }

    let meta = T::get_struct_meta();
    let cols = meta.cols;

    let mut vals_2d = Vec::with_capacity(vals.len());
    for d in vals {
        vals_2d.push(d.to_values());
    }

    update_raw(
        rivetx_sql,
        &table.into(),
        &cols,
        vals_2d,
        join_on,
        set_expr,
        max_per_batch,
        timeout,
    )
    .await
}

pub struct UpdateBuilder<T> {
    rivetx_sql: RivetxSql,
    table: RivetxString,
    data: Vec<T>,
    max_per_batch: usize,
    join_on: Vec<RivetxString>,
    set_expr: Vec<RivetxString>,
    timeout: Duration,
}

impl<T> UpdateBuilder<T>
where
    T: FromSqlRow + ToSqlValues + Send + Sync,
{
    pub fn new(rivetx_sql: &RivetxSql, table: impl Into<RivetxString>, data: Vec<T>) -> Self {
        Self {
            rivetx_sql: rivetx_sql.clone(),
            table: table.into(),
            data,
            max_per_batch: BATCH_SIZE,
            join_on: Vec::new(),
            set_expr: Vec::new(),
            timeout: TIMEOUT,
        }
    }

    pub fn batch_size(mut self, size: usize) -> Self {
        self.max_per_batch = size;
        self
    }

    pub fn join_on(mut self, cols: Vec<RivetxString>) -> Self {
        self.join_on = cols;
        self
    }

    pub fn set_expr(mut self, exprs: Vec<RivetxString>) -> Self {
        self.set_expr = exprs;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn exec(self) -> Result<UpdateResult> {
        update(
            &self.rivetx_sql,
            &self.table,
            &self.data,
            &self.join_on,
            &self.set_expr,
            self.max_per_batch,
            self.timeout,
        )
        .await
    }
}
