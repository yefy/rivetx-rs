use crate::conn::RivetxSql;
use crate::FromSqlRow;
use rivetx_core::rivetx_str::RivetxStr;
use std::time::{Duration, Instant};

fn quote_ident(name: &str) -> String {
    format!("`{}`", name.replace('`', "``"))
}

pub fn generate_create_database_sql(db_name: &str) -> String {
    format!("CREATE DATABASE IF NOT EXISTS {};", quote_ident(db_name))
}

async fn exec_create_database(
    rivetx_sql: &RivetxSql,
    db_name: &str,
    execution_timeout: Duration,
) -> anyhow::Result<()> {
    if db_name.is_empty() {
        return Err(anyhow::anyhow!("create_database: db name is empty"));
    }
    let sql = generate_create_database_sql(db_name);

    let start_time = Instant::now();
    let exec_start = Instant::now();
    rivetx_sql
        .exec(&sql, &[], execution_timeout)
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "dbName:{}, allTime:{}ms, execTime:{}ms, query:{}, err:{}",
                db_name,
                start_time.elapsed().as_millis(),
                exec_start.elapsed().as_millis(),
                sql,
                e
            )
        })?;

    log::debug!(
        "dbName:{}, allTime:{}ms, execTime:{}ms, query:{}",
        db_name,
        start_time.elapsed().as_millis(),
        exec_start.elapsed().as_millis(),
        sql
    );
    Ok(())
}

/// Split a full MySQL URL into `(server_url, db_name)`.
///
/// `mysql://user:password@localhost:3306/database`
/// -> (`mysql://user:password@localhost:3306`, `database`)
///
/// `mysql://user:password@localhost:3306/database?pool_max=10&compress=fast`
/// -> (`mysql://user:password@localhost:3306?pool_max=10&compress=fast`, `database`)
pub fn parse_create_database_url(url: &str) -> anyhow::Result<(String, String)> {
    let Some(scheme_end) = url.find("://") else {
        return Err(anyhow::anyhow!("create_database: invalid url"));
    };
    let rest = &url[scheme_end + 3..];
    let auth_end = rest
        .find(|c| c == '/' || c == '?' || c == '#')
        .unwrap_or(rest.len());
    let after_auth = &rest[auth_end..];
    let suffix_start = after_auth
        .find(|c| c == '?' || c == '#')
        .unwrap_or(after_auth.len());
    let path = after_auth[..suffix_start].trim_start_matches('/');
    let db_name = path.trim_end_matches('/');
    if db_name.is_empty() || db_name.contains('/') {
        return Err(anyhow::anyhow!("create_database: url has no database name"));
    }

    let connect_url = format!(
        "{}{}{}",
        &url[..scheme_end + 3],
        &rest[..auth_end],
        &after_auth[suffix_start..]
    );
    Ok((connect_url, db_name.to_string()))
}

/// Drop `pool_min` / `pool_max` so a one-shot connect can use the function args.
pub fn url_without_pool_bounds(url: &str) -> String {
    let Some((base, rest)) = url.split_once('?') else {
        return url.to_string();
    };
    let (query, fragment) = rest
        .split_once('#')
        .map(|(q, f)| (q, Some(f)))
        .unwrap_or((rest, None));
    let kept: Vec<&str> = query
        .split('&')
        .filter(|pair| {
            let key = pair.split_once('=').map(|(k, _)| k).unwrap_or(pair);
            key != "pool_min" && key != "pool_max"
        })
        .collect();
    let mut out = base.to_string();
    if !kept.is_empty() {
        out.push('?');
        out.push_str(&kept.join("&"));
    }
    if let Some(fragment) = fragment {
        out.push('#');
        out.push_str(fragment);
    }
    out
}

/// Connect with a URL that has no database (e.g. `mysql://user:pass@host:3306`),
/// create the database, then disconnect.
#[cfg(feature = "native")]
pub async fn create_database(
    url: &str,
    db_name: &str,
    execution_timeout: Duration,
) -> anyhow::Result<()> {
    let sql = RivetxSql::new(&url_without_pool_bounds(url), 1, 2)?;
    let result = exec_create_database(&sql, db_name, execution_timeout).await;
    let _ = sql.disconnect().await;
    result
}

/// Parse a full URL that includes the database name
/// (e.g. `mysql://user:pass@host:3306/database?pool_max=10`),
/// connect without selecting that schema, create it, then disconnect.
#[cfg(feature = "native")]
pub async fn create_database_from_url(
    url: &str,
    execution_timeout: Duration,
) -> anyhow::Result<()> {
    let (connect_url, db_name) = parse_create_database_url(url)?;
    create_database(&connect_url, &db_name, execution_timeout).await
}

/// Create a database using an existing connection (wasm / already-open pool).
pub async fn create_database_on(
    rivetx_sql: &RivetxSql,
    db_name: &str,
    execution_timeout: Duration,
) -> anyhow::Result<()> {
    exec_create_database(rivetx_sql, db_name, execution_timeout).await
}

pub fn generate_create_table_sql<T: FromSqlRow>(table_name: &RivetxStr) -> String {
    let meta = T::get_struct_meta();
    let mut query = format!("CREATE TABLE IF NOT EXISTS {} (", table_name);

    for i in 0..meta.cols.len() {
        let col = &meta.cols[i];
        let sql_type = &meta.sql_types[i];
        let mut fixed_attr = meta.fixed_attrs[i].clone();
        let auto = if meta.auto_col_map.get(col).copied().unwrap_or(false)
            && !fixed_attr.contains("CURRENT_TIMESTAMP")
        {
            "AUTO_INCREMENT"
        } else {
            ""
        };
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
    table_name: &RivetxStr<'_>,
    execution_timeout: Duration,
) -> anyhow::Result<()> {
    let sql = crate::create::generate_create_table_sql::<T>(table_name);

    let start_time = Instant::now();
    let exec_start = Instant::now();
    rivetx_sql
        .exec(&sql, &[], execution_timeout)
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
