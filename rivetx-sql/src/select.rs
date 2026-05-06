use crate::conn::RivetxSql;
use crate::sql_value::SqlValue;
use crate::util::{build_query, QueryCond, BATCH_SIZE, TIMEOUT};
use crate::{FromSqlRow, StructMeta};
use anyhow::{anyhow, Result};
use mysql_async::prelude::{FromRow, Queryable};
use rivetx_core::rivetx_string::RivetxString;
use std::time::{Duration, Instant};
use tokio::time::timeout;

pub trait OrderFieldSelectValue {
    fn order_field_select_value(&self) -> SqlValue;
}

pub async fn select_raw<T>(
    rivetx_sql: &RivetxSql,
    table: &RivetxString,
    join: &RivetxString,
    query_cond: &QueryCond,
    cond: &RivetxString,
    cond_args: &Vec<SqlValue>,
    order: &RivetxString,
    limit: usize,
    offset: usize,
    batch_size: usize,
    mut execution_timeout: Duration,
) -> Result<Vec<T>>
where
    T: FromSqlRow + FromRow + OrderFieldSelectValue + Send + Sync + 'static,
{
    let meta: StructMeta = T::get_struct_meta();
    let fields = &meta.cols;

    if (!order.is_empty() || limit > 0 || offset > 0) && query_cond.in_batch_size > 0 {
        return Err(anyhow!(
            "(order/limit/offset) not supported with in_batch_size > 0"
        ));
    }

    let in_batch_size =
        if order.is_empty() && limit == 0 && offset == 0 && query_cond.in_batch_size == 0 {
            BATCH_SIZE
        } else {
            query_cond.in_batch_size
        };

    let effective_in_batch_size = if in_batch_size == 0 {
        usize::MAX
    } else {
        in_batch_size
    };
    let effective_limit = if limit == 0 { usize::MAX } else { limit };
    let effective_batch_size = if batch_size == 0 {
        BATCH_SIZE
    } else {
        batch_size
    };
    if execution_timeout == Duration::from_secs(0) {
        execution_timeout = TIMEOUT;
    }

    if query_cond.fixed_cols.len() != query_cond.fixed_vals.len() {
        return Err(anyhow::anyhow!("fixedCols and fixedVals length mismatch"));
    }

    if query_cond.in_cols.len() > 0 {
        if query_cond.in_vals.len() == 0 {
            return Err(anyhow::anyhow!(
                "len(queryCond.InCols) > 0 && len(queryCond.InVals) == 0"
            ));
        } else {
            if query_cond.in_cols.len() != query_cond.in_vals[0].len() {
                return Err(anyhow::anyhow!(
                    "len(queryCond.InCols) != len(queryCond.InVals[0])"
                ));
            }
        }
    }

    for (i, vals) in query_cond.in_vals.iter().enumerate() {
        if vals.len() != query_cond.in_cols.len() {
            return Err(anyhow::anyhow!(
                "InVals[{}] length {} does not match InCols length {}",
                i,
                vals.len(),
                query_cond.in_cols.len()
            ));
        }
    }

    if query_cond.fixed_cols.len() <= 0 && query_cond.in_cols.len() <= 0 && cond.len() <= 0 {
        return Err(anyhow::anyhow!(
            "both FixedCols and InCols and cond are empty"
        ));
    }

    let mut chunks: Vec<Vec<Vec<SqlValue>>> = Vec::with_capacity(query_cond.in_vals.len() + 1);
    if !query_cond.in_vals.is_empty() {
        for chunk in query_cond.in_vals.chunks(effective_in_batch_size) {
            chunks.push(chunk.to_vec());
        }
    } else {
        chunks.push(Vec::new());
    }

    let mut result = Vec::with_capacity(std::cmp::min(effective_limit, 1024));

    let start_time = Instant::now();
    let mut total_count = 0;
    for (chunk_index, chunk) in chunks.into_iter().enumerate() {
        let mut data_offset = offset;
        let mut data_limit = effective_limit;

        loop {
            if data_limit == 0 {
                break;
            }

            let mut tuples = Vec::with_capacity(chunk.len() * 4);
            let mut args =
                Vec::with_capacity(query_cond.fixed_vals.len() + cond_args.len() + chunk.len());

            args.extend(query_cond.fixed_vals.clone());
            args.extend(cond_args.clone());

            for vals in &chunk {
                let placeholders = vec!["?"; vals.len()].join(",");
                tuples.push(format!("({})", placeholders).into());
                args.extend(vals.clone());
            }

            let fixed_conds: Vec<RivetxString> = query_cond
                .fixed_cols
                .iter()
                .map(|col| format!("{} = ?", col).into())
                .collect();

            let min_limit = std::cmp::min(data_limit, effective_batch_size);
            let limit_clause = format!(" LIMIT {} OFFSET {}", min_limit, data_offset);

            let query = build_query(
                &["SELECT", &fields.join(", "), "FROM"],
                table.as_str(),
                join.as_str(),
                &fixed_conds,
                cond.as_str(),
                &query_cond.in_cols,
                &tuples,
                order.as_str(),
                &limit_clause,
            );

            let exec_start = Instant::now();
            let mut conn = rivetx_sql.conn().await?;
            let rows: Vec<T> = timeout(execution_timeout, conn.exec(query, &args)).await
                .map_err(|e| anyhow::anyhow!( "batch_start:{}, data_offset: {}, allTime: {}ms, execTime: {}ms, totalAffected: {}, rowsAffected: {}, lastInsertId: {}, args:{:?}, err:{}",
                chunk_index,
                data_offset,
                start_time.elapsed().as_millis(),
                exec_start.elapsed().as_millis(),
                total_count,
                0,
                0,
                args, e))?
                .map_err(|e| anyhow::anyhow!( "batch_start:{}, data_offset: {}, allTime: {}ms, execTime: {}ms, totalAffected: {}, rowsAffected: {}, lastInsertId: {}, args:{:?}, err:{}",
                chunk_index,
                data_offset,
                start_time.elapsed().as_millis(),
                exec_start.elapsed().as_millis(),
                total_count,
                0,
                0,
                args, e))?;
            let batch_count = rows.len();

            result.extend(rows);

            total_count += batch_count;

            log::debug!(
                "batch_start:{}, data_offset: {}, allTime: {}ms, execTime: {}ms, totalAffected: {}, rowsAffected: {}, lastInsertId: {}, args:{:?}",
                chunk_index,
                data_offset,
                start_time.elapsed().as_millis(),
                exec_start.elapsed().as_millis(),
                total_count,
                batch_count,
                0,
                args
            );

            if batch_count < min_limit {
                break;
            }
            data_offset += min_limit;
            data_limit -= min_limit;
        }
    }

    Ok(result)
}

pub struct SelectBuilder<T> {
    rivetx_sql: RivetxSql,
    table: RivetxString,
    join: RivetxString,
    query_cond: QueryCond,
    cond: RivetxString,
    cond_args: Vec<SqlValue>,
    order: RivetxString,
    limit: usize,
    offset: usize,
    batch_size: usize,
    timeout: Duration,
    order_field: RivetxString,
    is_desc_order_field: bool,
    _marker: std::marker::PhantomData<T>,
}

impl<T> SelectBuilder<T>
where
    T: FromSqlRow + FromRow + OrderFieldSelectValue + Send + Sync + 'static,
{
    pub fn new(rivetx_sql: &RivetxSql, table: impl Into<RivetxString>) -> Self {
        Self {
            rivetx_sql: rivetx_sql.clone(),
            table: table.into(),
            join: RivetxString::default(),
            query_cond: QueryCond::new(),
            cond: RivetxString::default(),
            cond_args: Vec::new(),
            order: RivetxString::default(),
            limit: 0,
            offset: 0,
            batch_size: 0,
            timeout: TIMEOUT,
            order_field: RivetxString::default(),
            is_desc_order_field: false,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn join(mut self, join: impl Into<RivetxString>) -> Self {
        self.join = join.into();
        self
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

    pub fn where_in_batch_size(mut self, in_batch_size: usize) -> Self {
        self.query_cond.in_batch_size = in_batch_size;
        self
    }

    pub fn where_cond(mut self, cond: impl Into<RivetxString>, args: Vec<SqlValue>) -> Self {
        if !self.cond.is_empty() {
            self.cond.push_str(" ");
        }
        self.cond.push_str(cond.into().as_str());
        self.cond_args.extend(args);
        self
    }

    pub fn order(mut self, order: impl Into<RivetxString>) -> Self {
        self.order = order.into();
        self
    }
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    pub fn order_field_select(
        mut self,
        field: impl Into<RivetxString>,
        is_desc: bool,
        limit: usize,
    ) -> Self {
        self.order_field = field.into();
        self.is_desc_order_field = is_desc;
        self.limit = limit;
        self
    }

    async fn exec_order_field_select(&self, sort: &str, operator: &str) -> Result<Vec<T>> {
        let total_limit = self.limit;
        let batch_size = if self.batch_size <= 0 {
            BATCH_SIZE
        } else {
            self.batch_size
        };
        let mut result = Vec::with_capacity(total_limit);

        if self.offset > total_limit && total_limit > 0 {
            return Err(anyhow!(
                "offset:{} > total_limit:{}",
                self.offset,
                total_limit
            ));
        }

        let mode = if !self.query_cond.fixed_cols.is_empty() || !self.cond.is_empty() {
            " AND "
        } else {
            ""
        };
        let mut last_id: Option<SqlValue> = None;

        while result.len() < total_limit {
            let remaining = total_limit - result.len();
            let current_limit = std::cmp::min(remaining, batch_size);

            let (current_cond, current_args) = if let Some(id) = last_id {
                let mut c = self.cond.clone();
                let mut a = self.cond_args.clone();
                // 拼接游标条件: AND field > ?
                let cursor_cond = format!("{} {} {} ?", mode, self.order_field, operator);
                c.push_str(&cursor_cond);
                a.push(id);
                (c, a)
            } else {
                let mut c = self.cond.clone();
                let a = self.cond_args.clone();
                c.push_str(&format!(" {} 1=1", mode));
                (c, a)
            };

            let order_clause = format!("ORDER BY {} {}", self.order_field, sort);

            let current_offset = if result.is_empty() { self.offset } else { 0 };

            let values = select_raw::<T>(
                &self.rivetx_sql,
                &self.table,
                &self.join,
                &self.query_cond,
                &current_cond,
                &current_args,
                &order_clause.into(),
                current_limit,
                current_offset,
                batch_size,
                self.timeout,
            )
            .await?;

            if values.is_empty() {
                break;
            }

            last_id = Some(values.last().unwrap().order_field_select_value());
            result.extend(values);
        }

        Ok(result)
    }

    pub async fn exec(self) -> Result<Vec<T>> {
        if self.order_field.is_empty() {
            select_raw(
                &self.rivetx_sql,
                &self.table,
                &self.join,
                &self.query_cond,
                &self.cond,
                &self.cond_args,
                &self.order,
                self.limit,
                self.offset,
                self.batch_size,
                self.timeout,
            )
            .await
        } else {
            if self.is_desc_order_field {
                self.exec_order_field_select("DESC", "<").await
            } else {
                self.exec_order_field_select("ASC", ">").await
            }
        }
    }
}
