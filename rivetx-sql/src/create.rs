use crate::conn::RivetxSql;
use crate::FromSqlRow;
use mysql_async::prelude::Queryable;
use rivetx_core::rivetx_string::RivetxString;
use std::time::{Duration, Instant};
use tokio::time::timeout;

pub fn generate_create_table_sql<T: FromSqlRow>(table_name: &RivetxString) -> String {
    let meta = T::get_struct_meta();
    let mut query = format!("CREATE TABLE IF NOT EXISTS {} (", table_name);

    for i in 0..meta.cols.len() {
        let col = &meta.cols[i];
        let sql_type = &meta.sql_types[i];
        let auto = if meta.auto_col_map.get(col).copied().unwrap_or(false) {
            "AUTO_INCREMENT"
        } else {
            ""
        };

        let mut fixed_attr = meta.fixed_attrs[i].clone();
        if fixed_attr.is_empty() {
            fixed_attr = "NOT NULL".into()
        }

        query.push_str(&format!(" {} {} {} {}, ", col, sql_type, fixed_attr, auto));
    }

    if let Some(ref primary) = meta.primary {
        query.push_str(&format!(" PRIMARY KEY ({}), ", primary));
    }

    for (key, values) in &meta.unique_map {
        query.push_str(&format!(" UNIQUE INDEX {} ( ", key));
        let col_list = values.join(", ");
        query.push_str(&col_list);
        query.push_str("),");
    }

    for (key, value) in &meta.index_map {
        query.push_str(&format!(" INDEX {} ( {} ),", key, value));
    }

    let mut final_query = query.trim_end_matches(|c| c == ',' || c == ' ').to_string();
    final_query.push_str(");");

    final_query
}

pub async fn create_table<T: FromSqlRow>(
    rivetx_sql: &RivetxSql,
    table_name: &RivetxString,
    execution_timeout: Duration,
) -> anyhow::Result<()> {
    let sql = crate::create::generate_create_table_sql::<T>(table_name);

    let start_time = Instant::now();
    let mut conn = rivetx_sql.conn().await?;

    let exec_start = Instant::now();
    timeout(execution_timeout, conn.query_drop(&sql))
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "tableName:{}, allTime:{}ms, execTime:{}ms, query:{}, err:{}",
                table_name,
                start_time.elapsed().as_millis(),
                exec_start.elapsed().as_millis(),
                sql,
                e
            )
        })?
        .map_err(|e| {
            anyhow::anyhow!(
                "tableName:{}, allTime:{}ms, execTime:{}ms, query:{}, err:{}",
                table_name,
                start_time.elapsed().as_millis(),
                exec_start.elapsed().as_millis(),
                sql,
                e
            )
        })?;

    log::debug!(
        "tableName:{}, allTime:{}ms, execTime:{}ms, query:{}",
        table_name,
        start_time.elapsed().as_millis(),
        exec_start.elapsed().as_millis(),
        sql
    );
    Ok(())
}
