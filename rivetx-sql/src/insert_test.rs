use crate::conn::RivetxSql;
use crate::insert::{insert, insert_raw, InsertBuilder};
use crate::util_tests::{
    test_data_count_rows, test_data_create_table, test_data_query_all_no_id,
    test_data_truncate_table, zero_naive_date_time, TestData,
};
use anyhow::Context;
use anyhow::{anyhow, Result};
use chrono::Timelike;
use log::info;
use mysql_async::prelude::Queryable;
use mysql_async::Value;
use std::time::Duration;

/// Entry point: runs all insert tests in sequence.
pub async fn test_insert(rivetx_sql: &RivetxSql) -> Result<()> {
    test_batch_insert(rivetx_sql).await.here()?;
    test_batch_insert_struct(rivetx_sql).await.here()?;
    test_batch_insert_no_duplicate_update(rivetx_sql)
        .await
        .here()?;
    test_batch_insert_struct_no_duplicate_update(rivetx_sql)
        .await
        .here()?;
    test_batch_new_insert_struct(rivetx_sql).await.here()?;
    test_batch_new_insert_struct_point(rivetx_sql)
        .await
        .here()?;
    Ok(())
}

/// Test `insert_raw`: batch insert + `ON DUPLICATE KEY UPDATE`.
pub async fn test_batch_insert(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await.here()?;
    test_data_truncate_table(rivetx_sql).await.here()?;

    let cols = vec![
        "index_col".into(),
        "key_col".into(),
        "name_id".into(),
        "name_index".into(),
        "curr_time".into(),
        "created_at".into(),
        "updated_at".into(),
    ];

    let mut vals: Vec<Vec<Value>> = Vec::new();
    for i in 0..10 {
        vals.push(vec![
            Value::from(i),
            Value::from(if i < 3 { "abc" } else { "xyz" }),
            Value::from(i + 1),
            Value::from(1001 + i),
            chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap()
                .into(),
            zero_naive_date_time().into(),
            zero_naive_date_time().into(),
        ]);
    }

    let on_duplicate = "name_id = VALUES(name_id), name_index = name_index + VALUES(name_index)";

    insert_raw(
        rivetx_sql,
        &"test_data".into(),
        &cols,
        vals,
        2,
        &on_duplicate.into(),
        false,
        Duration::from_secs(10),
    )
    .await
    .here()?;

    let count = test_data_count_rows(rivetx_sql, "test_data").await;
    if count != 10 {
        return Err(anyhow!("expected 10 rows, got {}", count));
    }

    // Verify `ON DUPLICATE KEY UPDATE`.
    let vals_dup = vec![vec![
        Value::from(0),
        Value::from("abc"),
        Value::from(11),
        Value::from(11),
        chrono::Local::now()
            .naive_local()
            .with_nanosecond(0)
            .unwrap()
            .into(),
        zero_naive_date_time().into(),
        zero_naive_date_time().into(),
    ]];
    insert_raw(
        rivetx_sql,
        &"test_data".into(),
        &cols,
        vals_dup,
        2,
        &on_duplicate.into(),
        false,
        Duration::from_secs(10),
    )
    .await
    .here()?;

    let mut conn = rivetx_sql.get_conn().await.here()?;
    let row_res: Option<(i32, i32)> = conn
        .query_first(
            "SELECT name_id, name_index FROM test_data WHERE index_col = 0 AND key_col = 'abc'",
        )
        .await
        .here()?;

    if let Some((name_id, name_index)) = row_res {
        if name_id != 11 || name_index != 1012 {
            return Err(anyhow!(
                "ON DUPLICATE KEY UPDATE failed, got name_id={}, name_index={}",
                name_id,
                name_index
            ));
        }
    } else {
        return Err(anyhow!("row not found for index_col=0, key_col='abc'"));
    }

    info!("test_batch_insert passed  ");
    Ok(())
}

/// Test `InsertBuilder`: batch insert structs + `ON DUPLICATE KEY UPDATE`.
pub async fn test_batch_insert_struct(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await.here()?;
    test_data_truncate_table(rivetx_sql).await.here()?;

    let mut data = Vec::new();
    for i in 0..10 {
        data.push(TestData {
            id: 0,
            index: i as i32,
            key: (if i < 3 { "abc" } else { "xyz" }).to_string(),
            name_id: (i + 1) as i32,
            name_index: (1001 + i) as i32,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        });
    }

    let on_duplicate = "name_id = VALUES(name_id), name_index = name_index + VALUES(name_index)";
    InsertBuilder::new("test_data", data)
        .batch_size(2)
        .on_duplicate_update(on_duplicate)
        .timeout(Duration::from_secs(10))
        .exec(rivetx_sql)
        .await
        .here()?;

    let count = test_data_count_rows(rivetx_sql, "test_data").await;
    if count != 10 {
        return Err(anyhow!("expected 10 rows, got {}", count));
    }

    info!("test_batch_insert_struct passed  ");
    Ok(())
}

/// Test `insert_raw`: batch insert without `ON DUPLICATE KEY UPDATE`.
pub async fn test_batch_insert_no_duplicate_update(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await.here()?;
    test_data_truncate_table(rivetx_sql).await.here()?;

    let cols = vec![
        "index_col".into(),
        "key_col".into(),
        "name_id".into(),
        "name_index".into(),
        "curr_time".into(),
        "created_at".into(),
        "updated_at".into(),
    ];

    let mut vals: Vec<Vec<Value>> = Vec::new();
    for i in 0..10 {
        vals.push(vec![
            Value::from(i),
            Value::from(if i < 3 { "abc" } else { "xyz" }),
            Value::from(i + 1),
            Value::from(1001 + i),
            chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap()
                .into(),
            zero_naive_date_time().into(),
            zero_naive_date_time().into(),
        ]);
    }

    insert_raw(
        rivetx_sql,
        &"test_data".into(),
        &cols,
        vals,
        2,
        &"".into(),
        false,
        Duration::from_secs(10),
    )
    .await
    .here()?;

    let count = test_data_count_rows(rivetx_sql, "test_data").await;
    if count != 10 {
        return Err(anyhow!("expected 10 rows, got {}", count));
    }

    info!("test_batch_insert_no_duplicate_update passed  ");
    Ok(())
}

/// Test `InsertBuilder`: insert structs without `ON DUPLICATE KEY UPDATE`.
pub async fn test_batch_insert_struct_no_duplicate_update(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await.here()?;
    test_data_truncate_table(rivetx_sql).await.here()?;

    let data = vec![TestData {
        id: 0,
        index: 200,
        key: "struct_no_up".into(),
        name_id: 1,
        name_index: 1,
        curr_time: chrono::Local::now()
            .naive_local()
            .with_nanosecond(0)
            .unwrap(),
        created_at: zero_naive_date_time(),
        updated_at: zero_naive_date_time(),
    }];

    InsertBuilder::new("test_data", data)
        .exec(rivetx_sql)
        .await
        .here()?;

    let count = test_data_count_rows(rivetx_sql, "test_data").await;
    if count != 1 {
        return Err(anyhow!("expected 1 row, got {}", count));
    }

    info!("test_batch_insert_struct_no_duplicate_update passed  ");
    Ok(())
}

/// Test `InsertBuilder` + `insert`: batch insert structs and validate `last_insert_id`,
/// `total_affected`, and data consistency.
pub async fn test_batch_new_insert_struct(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await.here()?;
    test_data_truncate_table(rivetx_sql).await.here()?;

    let mut test_data = Vec::new();
    for i in 0..10 {
        test_data.push(TestData {
            id: 0,
            index: i as i32,
            key: (if i < 3 { "abc" } else { "xyz" }).to_string(),
            name_id: (i + 1) as i32,
            name_index: (1001 + i) as i32,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        });
    }

    let on_duplicate = "name_id = VALUES(name_id), name_index = name_index + VALUES(name_index)";
    let result = InsertBuilder::new("test_data", test_data.clone())
        .batch_size(2)
        .on_duplicate_update(on_duplicate)
        .timeout(Duration::from_secs(10))
        .exec(rivetx_sql)
        .await
        .here()?;

    info!("test_batch_new_insert_struct result:{:?}", result);

    // Validate `last_insert_id`.
    let mut conn = rivetx_sql.get_conn().await.here()?;
    let last_id: u64 = conn
        .query_first("SELECT id FROM test_data ORDER BY id DESC LIMIT 1")
        .await?
        .unwrap_or(0);

    if result.last_insert_id != last_id {
        return Err(anyhow!(
            "result.last_insert_id:{} != last_id:{}",
            result.last_insert_id,
            last_id
        ));
    }

    // Validate `total_affected`.
    let count = test_data_count_rows(rivetx_sql, "test_data").await;
    if count != 10 || count != result.total_affected as usize {
        return Err(anyhow!(
            "expected 10 rows, got count={}, total_affected={}",
            count,
            result.total_affected
        ));
    }

    // Validate data consistency.
    let db_rows = test_data_query_all_no_id(rivetx_sql).await.here()?;
    if db_rows.len() != test_data.len() {
        return Err(anyhow!(
            "len(db_rows):{} != len(test_data):{}",
            db_rows.len(),
            test_data.len()
        ));
    }

    for (i, row) in db_rows.iter().enumerate() {
        if row.index != test_data[i].index
            || row.key != test_data[i].key
            || row.name_id != test_data[i].name_id
            || row.name_index != test_data[i].name_index
        {
            return Err(anyhow!("row {} data mismatch", i));
        }
    }

    // Verify `ON DUPLICATE KEY UPDATE` via the `insert` function.
    let data_dup = vec![TestData {
        id: 0,
        index: 0,
        key: "abc".into(),
        name_id: 11,
        name_index: 11,
        curr_time: chrono::Local::now()
            .naive_local()
            .with_nanosecond(0)
            .unwrap(),
        created_at: zero_naive_date_time(),
        updated_at: zero_naive_date_time(),
    }];
    insert::<TestData>(
        rivetx_sql,
        &"test_data".into(),
        &data_dup,
        2,
        &on_duplicate.into(),
        false,
        Duration::from_secs(10),
    )
    .await
    .here()?;

    let row_res: Option<(i32, i32)> = conn
        .query_first(
            "SELECT name_id, name_index FROM test_data WHERE index_col = 0 AND key_col = 'abc'",
        )
        .await
        .here()?;
    if let Some((name_id, name_index)) = row_res {
        if name_id != 11 || name_index != 1012 {
            return Err(anyhow!(
                "ON DUPLICATE KEY UPDATE failed for struct, got name_id={}, name_index={}",
                name_id,
                name_index
            ));
        }
    }

    info!("test_batch_new_insert_struct passed  ");
    Ok(())
}

/// Test `InsertBuilder`: batch insert structs (pointer semantics) and validate `last_insert_id`
/// and data consistency.
pub async fn test_batch_new_insert_struct_point(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await.here()?;
    test_data_truncate_table(rivetx_sql).await.here()?;

    let mut test_data = Vec::new();
    for i in 0..10 {
        test_data.push(TestData {
            id: 0,
            index: i as i32,
            key: (if i < 3 { "abc" } else { "xyz" }).to_string(),
            name_id: (i + 1) as i32,
            name_index: (1001 + i) as i32,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        });
    }

    let on_duplicate = "name_id = VALUES(name_id), name_index = name_index + VALUES(name_index)";
    let result = InsertBuilder::new("test_data", test_data.clone())
        .batch_size(2)
        .on_duplicate_update(on_duplicate)
        .timeout(Duration::from_secs(10))
        .exec(rivetx_sql)
        .await
        .here()?;

    info!("test_batch_new_insert_struct_point result:{:?}", result);

    // Validate `last_insert_id`.
    let mut conn = rivetx_sql.get_conn().await.here()?;
    let last_id: u64 = conn
        .query_first("SELECT id FROM test_data ORDER BY id DESC LIMIT 1")
        .await?
        .unwrap_or(0);

    if result.last_insert_id != last_id {
        return Err(anyhow!(
            "result.last_insert_id:{} != last_id:{}",
            result.last_insert_id,
            last_id
        ));
    }

    // Validate row count.
    let count = test_data_count_rows(rivetx_sql, "test_data").await;
    if count != 10 {
        return Err(anyhow!("expected 10 rows, got {}", count));
    }

    // Validate data consistency.
    let rows = test_data_query_all_no_id(rivetx_sql).await.here()?;
    for (i, row) in rows.iter().enumerate() {
        if row.index != test_data[i].index {
            return Err(anyhow!(
                "row {} index mismatch: {} != {}",
                i,
                row.index,
                test_data[i].index
            ));
        }
    }

    info!("test_batch_new_insert_struct_point passed  ");
    Ok(())
}
