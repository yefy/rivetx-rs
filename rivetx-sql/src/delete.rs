use crate::conn::RivetxSql;
use crate::sql_value::SqlValue;
use crate::util::TIMEOUT;
use crate::util::{QueryCond, BATCH_SIZE};
use anyhow::Context;
use anyhow::{anyhow, Result};
use mysql_async::{prelude::*, Row, Value};
use rivetx_core::rivetx_str::RivetxStr;
use rivetx_core::rivetx_string::RivetxString;
use std::fmt::Write;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tokio::time::timeout;

#[derive(Debug, Default)]
pub struct DeleteResult {
    pub total_affected: u64,
    pub last_insert_id: u64,
}

pub async fn delete_raw(
    rivetx_sql: &RivetxSql,
    table: &RivetxStr<'_>,
    g: &QueryCond,
    cond: &RivetxStr<'_>,
    cond_args: &Vec<SqlValue>,
    limit: usize,
    mut execution_timeout: Duration,
) -> Result<DeleteResult> {
    if execution_timeout == Duration::from_secs(0) {
        execution_timeout = TIMEOUT;
    }

    let mut conn = rivetx_sql.get_conn().await.here()?;

    let mut total_affected = 0u64;
    let mut last_insert_id = 0u64;

    if g.fixed_cols.len() != g.fixed_vals.len() {
        return Err(anyhow!("fixed_cols and fixed_vals length mismatch"));
    }

    for (i, vals) in g.in_vals.iter().enumerate() {
        if vals.len() != g.in_cols.len() {
            return Err(anyhow!(
                "in_vals[{}] length {} does not match in_cols length {}",
                i,
                vals.len(),
                g.in_cols.len()
            ));
        }
    }

    if g.fixed_cols.is_empty() && g.in_cols.is_empty() && cond.is_empty() {
        return Err(anyhow!("both fixed_cols, in_cols and cond are empty"));
    }

    let in_batch_size = if g.in_batch_size == 0 {
        BATCH_SIZE
    } else {
        g.in_batch_size
    };

    if g.in_vals.len() > in_batch_size && limit > 0 {
        return Err(anyhow!("len(g.in_vals) > in_batch_size && limit > 0"));
    }

    let mut chunks: Vec<Vec<Vec<SqlValue>>> = Vec::with_capacity(g.in_vals.len() + 1);
    if !g.in_vals.is_empty() {
        for chunk in g.in_vals.chunks(in_batch_size) {
            chunks.push(chunk.to_vec());
        }
    } else {
        chunks.push(vec![]);
    }

    let start_time = Instant::now();
    for (_index, chunk) in chunks.into_iter().enumerate() {
        let mut args: Vec<SqlValue> =
            Vec::with_capacity(g.fixed_vals.len() + cond_args.len() + chunk.len());

        // 1. Fixed Vals
        if !g.fixed_vals.is_empty() {
            args.extend(g.fixed_vals.clone());
        }

        // 2. Cond Args
        if !cond_args.is_empty() {
            args.extend(cond_args.clone());
        }

        let mut sql = format!("DELETE FROM {}", table);
        let mut where_parts: Vec<RivetxStr<'_>> = Vec::with_capacity(128);

        // FixedCols
        for col in &g.fixed_cols {
            where_parts.push(format!("{} = ?", col).into());
        }

        // Cond
        if !cond.is_empty() {
            where_parts.push(cond.as_str().into());
        }

        // InCols
        if !g.in_cols.is_empty() && !chunk.is_empty() {
            let col_part: RivetxStr<'_> = if g.in_cols.len() > 1 {
                format!("({})", g.in_cols.join(", ")).into()
            } else {
                (&g.in_cols[0]).into()
            };

            let mut tuples = Vec::with_capacity(128);
            for row_vals in chunk {
                let placeholders = vec!["?"; row_vals.len()].join(", ");
                tuples.push(format!("({})", placeholders));
                args.extend(row_vals);
            }
            where_parts.push(format!("{} IN ({})", col_part, tuples.join(", ")).into());
        }

        if !where_parts.is_empty() {
            write!(sql, " WHERE {}", where_parts.join(" AND "))?;
        }

        if limit > 0 {
            write!(sql, " LIMIT {}", limit)?;
        }

        let exec_start = Instant::now();
        let result = timeout(execution_timeout, conn.exec_iter(&sql, &args)).await
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
        let affected = result.affected_rows();
        let last_id = result.last_insert_id().unwrap_or(0);

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

    Ok(DeleteResult {
        total_affected,
        last_insert_id,
    })
}

pub struct DeleteBuilder {
    table: RivetxString,
    query_cond: QueryCond,
    cond: RivetxString,
    cond_args: Vec<SqlValue>,
    limit: usize,
    timeout: Duration,
    reserve_field: RivetxString,
    reserve_size: usize,
    reserve_sleep: Duration,
}

impl DeleteBuilder {
    pub fn new(table: impl Into<RivetxString>) -> Self {
        Self {
            table: table.into(),
            query_cond: QueryCond::default(),
            cond: RivetxString::default(),
            cond_args: Vec::new(),
            limit: 0,
            timeout: TIMEOUT,
            reserve_field: RivetxString::default(),
            reserve_size: 0,
            reserve_sleep: Duration::from_secs(0),
        }
    }

    pub fn where_eq(mut self, col: impl Into<RivetxString>, val: impl Into<SqlValue>) -> Self {
        self.query_cond.fixed_cols.push(col.into());
        self.query_cond.fixed_vals.push(val.into());
        self
    }

    pub fn where_in(mut self, cols: Vec<RivetxString>, vals: Vec<Vec<SqlValue>>) -> Self {
        self.query_cond.in_cols = cols;
        self.query_cond.in_vals = vals;
        self
    }

    pub fn where_in_batch_size(mut self, size: usize) -> Self {
        self.query_cond.in_batch_size = size;
        self
    }

    pub fn where_raw(mut self, cond: impl Into<RivetxString>, args: Vec<SqlValue>) -> Self {
        if self.cond.is_empty() {
            self.cond = cond.into();
        } else {
            self.cond = format!("({}) AND ({})", self.cond, cond.into()).into();
        }
        self.cond_args.extend(args);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn reserve_size(
        mut self,
        field: impl Into<RivetxString>,
        size: usize,
        sleep: Duration,
    ) -> Self {
        self.reserve_field = field.into();
        self.reserve_size = size;
        self.reserve_sleep = sleep;
        self.limit = BATCH_SIZE;
        self
    }

    async fn exec_delete(&self, rivetx_sql: &RivetxSql) -> Result<DeleteResult> {
        delete_raw(
            rivetx_sql,
            &(&self.table).into(),
            &self.query_cond,
            &(&self.cond).into(),
            &self.cond_args,
            self.limit,
            self.timeout,
        )
        .await
    }

    async fn exec_reserve_size(&self, rivetx_sql: &RivetxSql) -> Result<DeleteResult> {
        let sql = format!(
            "SELECT {} FROM {} ORDER BY {} DESC LIMIT 1 OFFSET {}",
            self.reserve_field, self.table, self.reserve_field, self.reserve_size
        );

        let mut conn = rivetx_sql.get_conn().await.here()?;
        let row_opt: Option<Row> = conn.query_first(&sql).await.here()?;

        let key = match row_opt {
            Some(row) => row.get::<Value, usize>(0),
            None => return Ok(DeleteResult::default()),
        };

        if key.is_none() {
            return Ok(DeleteResult::default());
        }
        let key = SqlValue::from(key.unwrap());

        let mut res_final = DeleteResult::default();
        loop {
            let limit = if self.limit <= 0 {
                BATCH_SIZE
            } else {
                self.limit
            };

            let res = DeleteBuilder::new(self.table.as_str().to_string())
                .where_raw(format!("{} <= ?", self.reserve_field), vec![key.clone()])
                .limit(limit)
                .exec_delete(rivetx_sql)
                .await
                .here()?;

            if res.total_affected <= 0 {
                break;
            }

            res_final.total_affected += res.total_affected;
            res_final.last_insert_id = res.last_insert_id;

            if !self.reserve_sleep.is_zero() {
                sleep(self.reserve_sleep).await;
            }
        }

        Ok(res_final)
    }

    pub async fn exec(&self, rivetx_sql: &RivetxSql) -> Result<DeleteResult> {
        if self.reserve_field.is_empty() {
            self.exec_delete(rivetx_sql).await
        } else {
            self.exec_reserve_size(rivetx_sql).await
        }
    }
}
