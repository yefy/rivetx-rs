use crate::conn::RivetxSql;
use crate::insert::{insert_raw, InsertBuilder};
use crate::util_tests::*;
use anyhow::Context;
use chrono::Timelike;
use mysql_async::prelude::Queryable;
use mysql_async::Value;
use std::time::Duration;

pub async fn test_insert(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    test_batch_insert(rivetx_sql).await.here()?;
    test_batch_insert_struct(rivetx_sql).await.here()?;
    test_batch_insert_no_duplicate_update(rivetx_sql)
        .await
        .here()?;
    test_batch_insert_struct_no_duplicate_update(rivetx_sql)
        .await
        .here()?;
    test_batch_new_insert_struct(rivetx_sql).await.here()?;
    // In Rust, Vec<T> and Vec<Box<T>> are handled similarly; merged here for demonstration
    Ok(())
}

pub async fn test_batch_insert(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
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
            i.into(),
            (if i < 3 { "abc" } else { "xyz" }).into(),
            (i + 1).into(),
            (1001 + i).into(),
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

    assert_eq!(test_data_count_rows(rivetx_sql, "test_data").await, 10);

    let vals_dup = vec![vec![
        0.into(),
        "abc".into(),
        11.into(),
        11.into(),
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

    let mut conn = rivetx_sql.conn().await.here()?;
    let row: Option<(i32, i32)> = conn
        .query_first(
            "SELECT name_id, name_index FROM test_data WHERE index_col = 0 AND key_col = 'abc'",
        )
        .await
        .here()?;

    let (name_id, name_index) = row.unwrap();
    if name_id != 11 || name_index != 1012 {
        return Err(anyhow::anyhow!(
            "ON DUPLICATE KEY UPDATE failed, got name_id={}, name_index={}",
            name_id,
            name_index
        ));
    }

    log::info!("test_batch_insert passed  ");
    Ok(())
}

pub async fn test_batch_insert_struct(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
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
    InsertBuilder::new(rivetx_sql, "test_data", data)
        .batch_size(2)
        .on_duplicate_update(on_duplicate)
        .timeout(Duration::from_secs(10))
        .exec()
        .await
        .here()?;

    assert_eq!(test_data_count_rows(rivetx_sql, "test_data").await, 10);
    log::info!("test_batch_insert_struct passed  ");
    Ok(())
}

pub async fn test_batch_insert_no_duplicate_update(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
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
    let vals = vec![vec![
        100.into(),
        "unique".into(),
        1.into(),
        1.into(),
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
        vals,
        2,
        &"".into(),
        false,
        Duration::from_secs(10),
    )
    .await
    .here()?;
    assert_eq!(test_data_count_rows(rivetx_sql, "test_data").await, 1);

    log::info!("test_batch_insert_no_duplicate_update passed  ");
    Ok(())
}

pub async fn test_batch_insert_struct_no_duplicate_update(
    rivetx_sql: &RivetxSql,
) -> anyhow::Result<()> {
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

    InsertBuilder::new(rivetx_sql, "test_data", data)
        .exec()
        .await
        .here()?;
    assert_eq!(test_data_count_rows(rivetx_sql, "test_data").await, 1);

    log::info!("test_batch_insert_struct_no_duplicate_update passed  ");
    Ok(())
}

pub async fn test_batch_new_insert_struct(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
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
    let result = InsertBuilder::new(rivetx_sql, "test_data", test_data.clone())
        .batch_size(2)
        .on_duplicate_update(on_duplicate)
        .timeout(Duration::from_secs(10))
        .exec()
        .await
        .here()?;

    let mut conn = rivetx_sql.conn().await.here()?;
    let last_id: u64 = conn
        .query_first("SELECT id FROM test_data ORDER BY id DESC LIMIT 1")
        .await?
        .unwrap();
    assert_eq!(result.last_insert_id, last_id);
    assert_eq!(result.total_affected, 10);

    let db_rows = test_data_query_all_no_id(rivetx_sql).await.here()?;
    assert_eq!(db_rows.len(), test_data.len());

    for (i, row) in db_rows.iter().enumerate() {
        assert_eq!(row.index, test_data[i].index);
        assert_eq!(row.key, test_data[i].key);
    }

    log::info!("test_batch_new_insert_struct passed  ");
    Ok(())
}
