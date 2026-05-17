use crate::conn::RivetxSql;
use crate::create::create_table;
use crate::sql_value::SqlValue;
use crate::FromSqlRow;
use chrono::{NaiveDate, NaiveDateTime};
use mysql_async::prelude::*;
use mysql_async::Value;
use std::time::Duration;

#[derive(Default, Debug, Clone, PartialEq, FromSqlRow)]
pub struct TestData {
    #[attr(auto, primary)]
    #[db = "id"]
    pub id: u64,

    #[attr(unique = "u_td_ik", unique = "u_td_in")]
    #[db = "index_col"]
    pub index: i32,

    #[db = "key_col"]
    #[attr(unique = "u_td_ik")]
    #[size = "64"]
    pub key: String,

    #[db = "name_id"]
    #[attr(unique = "u_td_in")]
    pub name_id: i32,

    #[db = "name_index"]
    #[attr(index = "i_td_name_index")]
    pub name_index: i32,

    #[db = "curr_time"]
    pub curr_time: NaiveDateTime,

    #[db = "created_at"]
    #[attr("DEFAULT CURRENT_TIMESTAMP")]
    pub created_at: NaiveDateTime,

    #[db = "updated_at"]
    #[attr("DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP")]
    pub updated_at: NaiveDateTime,
}

impl crate::select::OrderFieldSelectValue for TestData {
    fn order_field_select_value(&self) -> SqlValue {
        Value::from(self.id).into()
    }
}

#[derive(Default, Debug, FromSqlRow)]
pub struct TestDataNoExport {
    #[db = "-"]
    pub index: i32,
    #[db = "key_col"]
    pub key: String,
    #[db = "name_id"]
    pub name_id: i32,
    #[db = "-"]
    pub name_index: i32,
}

impl crate::select::OrderFieldSelectValue for TestDataNoExport {
    fn order_field_select_value(&self) -> SqlValue {
        Value::from(0).into()
    }
}

#[derive(Default, Debug, FromSqlRow)]
pub struct TestDataByD {
    #[db = "d.index_col"]
    pub index: i32,
    #[db = "d.key_col"]
    pub key: String,
    #[db = "d.name_id"]
    pub name_id: i32,
    #[db = "d.name_index"]
    pub name_index: i32,
}

impl crate::select::OrderFieldSelectValue for TestDataByD {
    fn order_field_select_value(&self) -> SqlValue {
        Value::from(0).into()
    }
}

#[derive(Default, Debug, FromSqlRow)]
pub struct TestDataByAs {
    #[db = "d.index_col"]
    pub index: i32,
    #[db = "d.key_col"]
    pub key: String,
    #[db = "d.name_id"]
    pub name_id: i32,
    #[db = "d.name_id as name_id2"]
    pub name_id2: i32,
    #[db = "d.name_index"]
    pub name_index: i32,
}

impl crate::select::OrderFieldSelectValue for TestDataByAs {
    fn order_field_select_value(&self) -> SqlValue {
        Value::from(0).into()
    }
}

#[derive(Default, Debug, FromSqlRow)]
pub struct Testkey {
    #[attr(auto, primary)]
    #[db = "id"]
    pub id: u64,

    #[db = "index_col"]
    pub index: i32,
    #[db = "key_col"]
    pub key: String,

    #[db = "created_at"]
    #[attr("DEFAULT CURRENT_TIMESTAMP")]
    pub created_at: NaiveDateTime,

    #[db = "updated_at"]
    #[attr("DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP")]
    pub updated_at: NaiveDateTime,
}

impl crate::select::OrderFieldSelectValue for Testkey {
    fn order_field_select_value(&self) -> SqlValue {
        Value::from(0).into()
    }
}

pub async fn test_open_rivetx_sql() -> anyhow::Result<RivetxSql> {
    test_open_rivetx_sql_sync()
}

pub fn test_open_rivetx_sql_sync() -> anyhow::Result<RivetxSql> {
    //let mysql_url = "mysql://root:Yfygz@389@192.168.80.139:3306/test_db".to_string();
    let mysql_url = "mysql://root:Yfygz@389@192.168.192.139:3306/test_db".to_string();
    let max_open_conns = 10;
    let max_idle_conns = 5;
    let rivetx_sql = RivetxSql::new(&mysql_url, max_idle_conns, max_open_conns)?;
    Ok(rivetx_sql)
}

pub async fn test_data_create_table(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    create_table::<TestData>(rivetx_sql, &"test_data".into(), Duration::from_secs(5)).await?;
    Ok(())
}

pub async fn test_key_create_table(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    create_table::<Testkey>(rivetx_sql, &"test_key".into(), Duration::from_secs(5)).await?;
    Ok(())
}

pub async fn test_key_clear_table(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    let mut conn = rivetx_sql.conn().await?;
    conn.query_drop("TRUNCATE TABLE test_key;").await?;
    Ok(())
}

pub async fn test_key_drop_table(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    let mut conn = rivetx_sql.conn().await?;
    let _ = conn.query_drop("DROP TABLE test_key;").await;
    Ok(())
}

pub async fn test_data_clear_table(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    test_data_truncate_table(rivetx_sql).await
}

pub async fn test_data_truncate_table(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    let mut conn = rivetx_sql.conn().await?;
    conn.query_drop("TRUNCATE TABLE test_data;").await?;
    Ok(())
}

pub async fn test_data_drop_table(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    let mut conn = rivetx_sql.conn().await?;
    let _ = conn.query_drop("DROP TABLE test_data;").await;
    Ok(())
}

pub async fn test_data_count_rows(rivetx_sql: &RivetxSql, _table_name: &str) -> usize {
    let mut conn = match rivetx_sql.conn().await {
        Ok(c) => c,
        Err(_) => return 0,
    };

    let count: Option<usize> = match conn.query_first("SELECT COUNT(*) FROM test_data").await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("countRows error: {:?}", e);
            None
        }
    };
    count.unwrap_or(0)
}

pub async fn test_data_query_all(rivetx_sql: &RivetxSql) -> anyhow::Result<Vec<TestData>> {
    let mut conn = rivetx_sql.conn().await?;
    let query = "SELECT id, index_col, key_col, name_id, name_index, curr_time, created_at, updated_at FROM test_data ORDER BY index_col, key_col";

    log::info!("sql:{}", query);
    let result = conn
        .query_map(
            query,
            |(id, index, key, name_id, name_index, curr_time, created_at, updated_at)| TestData {
                id,
                index,
                key,
                name_id,
                name_index,
                curr_time,
                created_at,
                updated_at,
            },
        )
        .await?;

    Ok(result)
}

pub async fn test_data_query_all_no_id(rivetx_sql: &RivetxSql) -> anyhow::Result<Vec<TestData>> {
    let mut conn = rivetx_sql.conn().await?;
    let query =
        "SELECT index_col, key_col, name_id, name_index, curr_time FROM test_data ORDER BY index_col, key_col";
    log::info!("sql:{}", query);
    let result = conn
        .query_map(query, |(index, key, name_id, name_index, curr_time)| {
            TestData {
                id: 0, // Simulate Scan ignoring Id
                index,
                key,
                name_id,
                name_index,
                curr_time,
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            }
        })
        .await?;

    Ok(result)
}

pub async fn test_data_query_all_by_id(
    rivetx_sql: &RivetxSql,
    is_desc: bool,
    limit: usize,
) -> anyhow::Result<Vec<TestData>> {
    let order = if is_desc { "DESC" } else { "" };
    let query = format!(
        "SELECT id, index_col, key_col, name_id, name_index, curr_time, created_at, updated_at FROM test_data ORDER BY id {} LIMIT {}",
        order, limit
    );
    log::info!("sql:{}", query);

    let mut conn = rivetx_sql.conn().await?;
    let result = conn
        .query_map(
            query,
            |(id, index, key, name_id, name_index, curr_time, created_at, updated_at)| TestData {
                id,
                index,
                key,
                name_id,
                name_index,
                curr_time,
                created_at,
                updated_at,
            },
        )
        .await?;

    Ok(result)
}

pub fn zero_naive_date_time() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(1000, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}
