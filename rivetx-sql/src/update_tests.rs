use crate::conn::RivetxSql;
use crate::insert::insert;
use crate::update::{update, UpdateBuilder};
use crate::util_tests::{
    test_data_clear_table, test_data_create_table, test_data_query_all_no_id, zero_naive_date_time,
    TestData,
};
use anyhow::{anyhow, Result};
use chrono::Timelike;
use std::time::Duration;

pub async fn test_update(rivetx_sql: &RivetxSql) -> Result<()> {
    test_batch_update_struct(rivetx_sql).await?;
    test_batch_update_struct2(rivetx_sql).await?;
    test_batch_update_struct2_point(rivetx_sql).await?;
    Ok(())
}

async fn test_batch_update_struct(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await?;
    test_data_clear_table(rivetx_sql).await?;

    let test_data = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 1,
            name_index: 1000,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 1,
            key: "xyz".into(),
            name_id: 2,
            name_index: 2000,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 2,
            key: "def".into(),
            name_id: 3,
            name_index: 3000,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
    ];

    insert(
        rivetx_sql,
        &"test_data".into(),
        &test_data,
        2,
        &"".into(),
        false,
        Duration::from_secs(10),
    )
    .await?;

    let updates = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 10,
            name_index: 10,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        }, // name_id = 10, name_index += 10
        TestData {
            id: 0,
            index: 1,
            key: "xyz".into(),
            name_id: 20,
            name_index: 20,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        }, // name_id = 20, name_index += 20
        TestData {
            id: 0,
            index: 2,
            key: "def".into(),
            name_id: 30,
            name_index: 30,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        }, // name_id = 30, name_index += 30
    ];

    let join_on = vec!["index_col".into(), "key_col".into()];
    let set_expr = vec![
        "u.name_id = v.name_id".into(),
        "u.name_index = u.name_index + v.name_index".into(),
    ];

    update(
        rivetx_sql,
        &"test_data".into(),
        &updates,
        &join_on,
        &set_expr,
        2,
        Duration::from_secs(30),
    )
    .await?;

    let rows = test_data_query_all_no_id(rivetx_sql).await?;
    let expected = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 10,
            name_index: 1010,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 1,
            key: "xyz".into(),
            name_id: 20,
            name_index: 2020,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 2,
            key: "def".into(),
            name_id: 30,
            name_index: 3030,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
    ];

    if rows.len() != expected.len() {
        return Err(anyhow!(
            "len(rows) {} != len(expected) {}",
            rows.len(),
            expected.len()
        ));
    }

    for (i, row) in rows.iter().enumerate() {
        if row != &expected[i] {
            return Err(anyhow!(
                "row {} mismatch: got {:?}, want {:?}",
                i,
                row,
                expected[i]
            ));
        }
    }

    log::info!("BatchUpdateStruct test passed ✅");
    Ok(())
}

async fn test_batch_update_struct2(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await?;
    test_data_clear_table(rivetx_sql).await?;

    let test_data = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 1,
            name_index: 1000,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 1,
            key: "xyz".into(),
            name_id: 2,
            name_index: 2000,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 2,
            key: "def".into(),
            name_id: 3,
            name_index: 3000,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
    ];
    insert(
        rivetx_sql,
        &"test_data".into(),
        &test_data,
        2,
        &"".into(),
        false,
        Duration::from_secs(10),
    )
    .await?;

    let updates = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 10,
            name_index: 10,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 1,
            key: "xyz".into(),
            name_id: 20,
            name_index: 20,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 2,
            key: "def".into(),
            name_id: 30,
            name_index: 30,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
    ];

    let join_on = vec!["index_col".into(), "key_col".into()];
    let set_expr = vec![
        "u.name_id = v.name_id".into(),
        "u.name_index = u.name_index + v.name_index".into(),
    ];

    // 使用 Builder 模式执行
    let update_len = updates.len() as u64;
    let res = UpdateBuilder::new(rivetx_sql, "test_data", updates)
        .join_on(join_on)
        .set_expr(set_expr)
        .exec()
        .await?;

    if res.total_affected != update_len {
        return Err(anyhow!(
            "res.total_affected {} != expected {}",
            res.total_affected,
            update_len
        ));
    }

    let rows = test_data_query_all_no_id(rivetx_sql).await?;
    let expected = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 10,
            name_index: 1010,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 1,
            key: "xyz".into(),
            name_id: 20,
            name_index: 2020,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 2,
            key: "def".into(),
            name_id: 30,
            name_index: 3030,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
    ];

    for (i, row) in rows.iter().enumerate() {
        if row != &expected[i] {
            return Err(anyhow!(
                "row {} mismatch: got {:?}, want {:?}",
                i,
                row,
                expected[i]
            ));
        }
    }

    log::info!("TestBatchUpdateStruct2 test passed ✅");
    Ok(())
}

async fn test_batch_update_struct2_point(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await?;
    test_data_clear_table(rivetx_sql).await?;

    // Rust 中处理指针通常直接使用 Vec 对象，这里逻辑与 Struct2 保持一致
    // 插入初始数据
    let test_data = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 1,
            name_index: 1000,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 1,
            key: "xyz".into(),
            name_id: 2,
            name_index: 2000,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 2,
            key: "def".into(),
            name_id: 3,
            name_index: 3000,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
    ];
    insert(
        rivetx_sql,
        &"test_data".into(),
        &test_data,
        2,
        &"".into(),
        false,
        Duration::from_secs(10),
    )
    .await?;

    let updates = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 10,
            name_index: 10,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 1,
            key: "xyz".into(),
            name_id: 20,
            name_index: 20,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 2,
            key: "def".into(),
            name_id: 30,
            name_index: 30,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
    ];

    let join_on = vec!["index_col".into(), "key_col".into()];
    let set_expr = vec![
        "u.name_id = v.name_id".into(),
        "u.name_index = u.name_index + v.name_index".into(),
    ];

    let update_len = updates.len() as u64;
    let res = UpdateBuilder::new(rivetx_sql, "test_data", updates)
        .join_on(join_on)
        .set_expr(set_expr)
        .exec()
        .await?;

    if res.total_affected != update_len {
        return Err(anyhow!(
            "res.total_affected {} != expected {}",
            res.total_affected,
            update_len
        ));
    }

    let rows = test_data_query_all_no_id(rivetx_sql).await?;
    let expected = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 10,
            name_index: 1010,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 1,
            key: "xyz".into(),
            name_id: 20,
            name_index: 2020,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 2,
            key: "def".into(),
            name_id: 30,
            name_index: 3030,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
    ];

    for (i, row) in rows.iter().enumerate() {
        if row != &expected[i] {
            return Err(anyhow!(
                "row {} mismatch: got {:?}, want {:?}",
                i,
                row,
                expected[i]
            ));
        }
    }

    log::info!("TestBatchUpdateStruct2Point test passed ✅");
    Ok(())
}
