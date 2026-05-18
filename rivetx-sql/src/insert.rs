use anyhow::Context;
use crate::conn::RivetxSql;
use crate::util::{BATCH_SIZE, TIMEOUT};
use crate::FromSqlRow;
use mysql_async::prelude::Queryable;
use mysql_async::Value;
use rivetx_core::rivetx_string::RivetxString;
use std::time::{Duration, Instant};
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct InsertResult {
    pub total_affected: u64,
    pub last_insert_id: u64,
}

pub async fn insert_raw(
    rivetx_sql: &RivetxSql,
    table: &RivetxString,
    cols: &[RivetxString],
    mut vals: Vec<Vec<Value>>,
    mut max_per_batch: usize,
    on_duplicate_update: &RivetxString,
    ignore_duplicate: bool,
    mut execution_timeout: Duration,
) -> anyhow::Result<InsertResult> {
    if vals.is_empty() || cols.is_empty() {
        return Err(anyhow::anyhow!("vals or cols is empty"));
    }

    for (i, vals) in vals.iter().enumerate() {
        if vals.len() != cols.len() {
            return Err(anyhow::anyhow!(
                "InVals[{}] length {} does not match InCols length {}",
                i,
                vals.len(),
                cols.len()
            ));
        }
    }

    if execution_timeout == Duration::from_secs(0) {
        execution_timeout = TIMEOUT;
    }

    if max_per_batch <= 0 {
        max_per_batch = BATCH_SIZE;
    }

    let mut total_affected = 0u64;
    let mut last_insert_id = 0u64;
    let start_time = Instant::now();

    let insert_keyword = if ignore_duplicate {
        "INSERT IGNORE"
    } else {
        "INSERT"
    };

    while !vals.is_empty() {
        let chunk: Vec<_> = vals.drain(..max_per_batch.min(vals.len())).collect();
        let mut placeholders = String::with_capacity(chunk.len() * 4);
        let mut flat_args = Vec::with_capacity(chunk.len());

        for row in chunk {
            if row.len() != cols.len() {
                return Err(anyhow::anyhow!(
                    "Row length {} does not match column length {}",
                    row.len(),
                    cols.len()
                ));
            }
            placeholders.push_str("(");
            placeholders.push_str(&vec!["?"; row.len()].join(","));
            placeholders.push_str("),");
            flat_args.extend(row);
        }
        let placeholders = placeholders.trim_end_matches(',');

        let mut query = format!(
            "{} INTO {} ({}) VALUES {}",
            insert_keyword,
            table,
            cols.join(", "),
            placeholders
        );

        if !on_duplicate_update.is_empty() {
            query.push_str(" ON DUPLICATE KEY UPDATE ");
            query.push_str(on_duplicate_update.as_str());
        }

        let mut conn = rivetx_sql.conn().await.here()?;
        let exec_start = Instant::now();

        timeout(execution_timeout, conn.exec_drop(&query, &flat_args)).await
            .map_err(|e| anyhow::anyhow!(  "batch_start: {}, allTime: {}ms, execTime: {}ms, totalAffected: {}, rowsAffected: {}, lastInsertId: {}, args:{:?}, err:{}",
            total_affected - 0,
            start_time.elapsed().as_millis(),
            exec_start.elapsed().as_millis(),
            total_affected,
            0,
            last_insert_id,
            flat_args, e))?
            .map_err(|e| anyhow::anyhow!(  "batch_start: {}, allTime: {}ms, execTime: {}ms, totalAffected: {}, rowsAffected: {}, lastInsertId: {}, args:{:?}, err:{}",
            total_affected - 0,
            start_time.elapsed().as_millis(),
            exec_start.elapsed().as_millis(),
            total_affected,
            0,
            last_insert_id,
            flat_args, e))?;

        let affected = conn.affected_rows();
        let last_id = conn.last_insert_id().unwrap_or(0);

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
            flat_args
        );
    }

    Ok(InsertResult {
        total_affected,
        last_insert_id,
    })
}

pub async fn insert<T: FromSqlRow + crate::ToSqlValues>(
    rivetx_sql: &RivetxSql,
    table: &RivetxString,
    data: &Vec<T>,
    max_per_batch: usize,
    on_duplicate_update: &RivetxString,
    ignore_duplicate: bool,
    execution_timeout: Duration,
) -> anyhow::Result<InsertResult> {
    if data.is_empty() {
        return Err(anyhow::anyhow!("data is empty"));
    }

    let meta = T::get_struct_meta();
    let cols = meta.discard_auto_cols;

    let mut vals = Vec::with_capacity(data.len() * 20);
    for item in data {
        vals.push(item.to_values_discard_auto());
    }

    insert_raw(
        rivetx_sql,
        table,
        &cols,
        vals,
        max_per_batch,
        on_duplicate_update,
        ignore_duplicate,
        execution_timeout,
    )
    .await
}

pub struct InsertBuilder<T: FromSqlRow + crate::ToSqlValues> {
    rivetx_sql: RivetxSql,
    table: RivetxString,
    data: Vec<T>,
    max_per_batch: usize,
    on_duplicate_update: RivetxString,
    ignore_duplicate: bool,
    timeout: Duration,
}

impl<T: FromSqlRow + crate::ToSqlValues> InsertBuilder<T> {
    pub fn new(rivetx_sql: &RivetxSql, table: impl Into<RivetxString>, data: Vec<T>) -> Self {
        Self {
            rivetx_sql: rivetx_sql.clone(),
            table: table.into(),
            data,
            max_per_batch: BATCH_SIZE,
            on_duplicate_update: RivetxString::from(""),
            ignore_duplicate: false,
            timeout: TIMEOUT,
        }
    }

    pub fn batch_size(mut self, size: usize) -> Self {
        self.max_per_batch = size;
        self.self_now()
    }

    pub fn on_duplicate_update(mut self, update: impl Into<RivetxString>) -> Self {
        self.on_duplicate_update = update.into();
        self.self_now()
    }

    pub fn ignore_duplicate(mut self) -> Self {
        self.ignore_duplicate = true;
        self.self_now()
    }

    pub fn timeout(mut self, t: Duration) -> Self {
        self.timeout = t;
        self.self_now()
    }

    fn self_now(self) -> Self {
        self
    }

    pub async fn exec(self) -> anyhow::Result<InsertResult> {
        insert(
            &self.rivetx_sql,
            &self.table,
            &self.data,
            self.max_per_batch,
            &self.on_duplicate_update,
            self.ignore_duplicate,
            self.timeout,
        )
        .await
    }
}
