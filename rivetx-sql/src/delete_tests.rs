use crate::conn::RivetxSql;
use crate::delete::{delete_raw, DeleteBuilder};
use crate::insert::insert;
use crate::util::QueryCond;
use crate::util_tests::{
    test_data_clear_table, test_data_count_rows, test_data_create_table, test_data_query_all_no_id,
    zero_naive_date_time, TestData,
};
use anyhow::Context;
use anyhow::{anyhow, Result};
use chrono::Timelike;
use rivetx_core::rivetx_string::RivetxString;
use std::time::Duration;

pub async fn test_delete(rivetx_sql: &RivetxSql) -> Result<()> {
    test_batch_delete_per_group(rivetx_sql).await.here()?;
    test_batch_delete_per_group_struct2(rivetx_sql)
        .await
        .here()?;
    test_batch_delete_per_group_struct2_point_limit(rivetx_sql)
        .await
        .here()?;
    test_batch_delete_per_group_struct2_point_reserve(rivetx_sql)
        .await
        .here()?;
    Ok(())
}

async fn test_batch_delete_per_group(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await.here()?;
    test_data_clear_table(rivetx_sql).await.here()?;

    let test_data = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 1,
            name_index: 1001,
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
            key: "abc".into(),
            name_id: 2,
            name_index: 1002,
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
            key: "abc".into(),
            name_id: 3,
            name_index: 1003,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 3,
            key: "xyz".into(),
            name_id: 4,
            name_index: 1004,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 4,
            key: "xyz".into(),
            name_id: 5,
            name_index: 1005,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 5,
            key: "xyz".into(),
            name_id: 6,
            name_index: 1006,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 6,
            key: "xyz".into(),
            name_id: 7,
            name_index: 1007,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 7,
            key: "xyz".into(),
            name_id: 8,
            name_index: 1008,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 8,
            key: "xyz".into(),
            name_id: 9,
            name_index: 1009,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 9,
            key: "xyz".into(),
            name_id: 10,
            name_index: 1010,
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
        &RivetxString::from("test_data"),
        &test_data,
        2,
        &RivetxString::from(""),
        false,
        std::time::Duration::from_secs(10),
    )
    .await
    .here()?;

    if test_data_count_rows(rivetx_sql, "test_data").await != 10 {
        return Err(anyhow!("expected 10 rows"));
    }

    let groups = vec![
        QueryCond {
            fixed_cols: vec!["index_col".into(), "key_col".into()],
            fixed_vals: vec![0.into(), "abc".into()],
            in_cols: vec!["name_id".into(), "name_index".into()],
            in_vals: vec![
                vec![1.into(), 1001.into()],
                vec![2.into(), 1002.into()],
                vec![3.into(), 1003.into()],
                vec![4.into(), 1004.into()],
                vec![5.into(), 1005.into()],
            ],
            ..Default::default()
        },
        QueryCond {
            in_cols: vec!["name_id".into(), "name_index".into()],
            in_vals: vec![
                vec![6.into(), 1006.into()],
                vec![7.into(), 1007.into()],
                vec![8.into(), 1008.into()],
                vec![9.into(), 1009.into()],
                vec![10.into(), 1010.into()],
            ],
            ..Default::default()
        },
        QueryCond {
            fixed_cols: vec!["index_col".into(), "key_col".into()],
            fixed_vals: vec![1.into(), "xyz".into()],
            ..Default::default()
        },
    ];

    for group in groups {
        delete_raw(
            rivetx_sql,
            &"test_data".into(),
            &group,
            &"".into(),
            &Vec::new(),
            0,
            Duration::from_secs(0),
        )
        .await
        .here()?;
    }

    log::info!("BatchDeletePerGroup test passed  ");
    Ok(())
}

async fn test_batch_delete_per_group_struct2(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await.here()?;
    test_data_clear_table(rivetx_sql).await.here()?;

    let test_data = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 1,
            name_index: 1001,
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
            key: "abc".into(),
            name_id: 2,
            name_index: 1002,
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
            key: "abc".into(),
            name_id: 3,
            name_index: 1003,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 3,
            key: "xyz".into(),
            name_id: 4,
            name_index: 1004,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 4,
            key: "xyz".into(),
            name_id: 5,
            name_index: 1005,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 5,
            key: "xyz".into(),
            name_id: 6,
            name_index: 1006,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 6,
            key: "xyz".into(),
            name_id: 7,
            name_index: 1007,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 7,
            key: "xyz".into(),
            name_id: 8,
            name_index: 1008,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 8,
            key: "xyz".into(),
            name_id: 9,
            name_index: 1009,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 9,
            key: "xyz".into(),
            name_id: 10,
            name_index: 1010,
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
        &RivetxString::from("test_data"),
        &test_data,
        2,
        &RivetxString::from(""),
        false,
        std::time::Duration::from_secs(10),
    )
    .await
    .here()?;

    if test_data_count_rows(rivetx_sql, "test_data").await != 10 {
        return Err(anyhow!("expected 10 rows"));
    }

    {
        let res = DeleteBuilder::new("test_data")
            .where_eq("index_col", 0)
            .where_eq("key_col", "abc")
            .where_in(
                vec!["name_id".into(), "name_index".into()],
                vec![vec![1.into(), 1001.into()], vec![2.into(), 1002.into()]],
            )
            .exec(rivetx_sql)
            .await
            .here()?;
        if res.total_affected != 1 {
            return Err(anyhow!("total_affected mismatch"));
        }

        if test_data_count_rows(rivetx_sql, "test_data").await != 9 {
            return Err(anyhow!("expected 10 rows"));
        }
    }

    {
        let res = DeleteBuilder::new("test_data")
            .where_in(
                vec!["name_id".into(), "name_index".into()],
                vec![vec![4.into(), 1004.into()], vec![5.into(), 1005.into()]],
            )
            .exec(rivetx_sql)
            .await
            .here()?;

        if res.total_affected != 2 {
            return Err(anyhow::anyhow!(
                "res.total_affected != 2, got {}",
                res.total_affected
            ));
        }

        let count = test_data_count_rows(rivetx_sql, "test_data").await;
        if count != 7 {
            return Err(anyhow::anyhow!("expected 7 rows left, got {}", count));
        }
    }

    {
        let res = DeleteBuilder::new("test_data")
            .where_eq("index_col", 1)
            .where_eq("key_col", "xyz")
            .exec(rivetx_sql)
            .await
            .here()?;

        if res.total_affected != 0 {
            return Err(anyhow::anyhow!(
                "res.total_affected != 0, got {}",
                res.total_affected
            ));
        }

        let count = test_data_count_rows(rivetx_sql, "test_data").await;
        if count != 7 {
            return Err(anyhow::anyhow!("expected 7 rows left, got {}", count));
        }
    }

    log::info!("BatchDeletePerGroupStruct2 test passed  ");
    Ok(())
}

async fn test_batch_delete_per_group_struct2_point_limit(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await.here()?;
    test_data_clear_table(rivetx_sql).await.here()?;

    let test_data = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 1,
            name_index: 1001,
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
            key: "abc".into(),
            name_id: 2,
            name_index: 1002,
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
            key: "abc".into(),
            name_id: 3,
            name_index: 1003,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 3,
            key: "xyz".into(),
            name_id: 4,
            name_index: 1004,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 4,
            key: "xyz".into(),
            name_id: 5,
            name_index: 1005,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 5,
            key: "xyz".into(),
            name_id: 6,
            name_index: 1006,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 6,
            key: "xyz".into(),
            name_id: 7,
            name_index: 1007,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 7,
            key: "xyz".into(),
            name_id: 8,
            name_index: 1008,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 8,
            key: "xyz".into(),
            name_id: 9,
            name_index: 1009,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 0,
            index: 9,
            key: "xyz".into(),
            name_id: 10,
            name_index: 1010,
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
        std::time::Duration::from_secs(10),
    )
    .await
    .here()?;

    if test_data_count_rows(rivetx_sql, "test_data").await != 10 {
        return Err(anyhow!("expected 10 rows"));
    }

    let res = DeleteBuilder::new("test_data")
        .where_in(
            vec!["name_id".into(), "name_index".into()],
            vec![vec![4.into(), 1004.into()], vec![5.into(), 1005.into()]],
        )
        .limit(1)
        .exec(rivetx_sql)
        .await
        .here()?;

    if res.total_affected != 1 {
        return Err(anyhow!("limit(1) should affect 1 row"));
    }
    Ok(())
}

async fn test_batch_delete_per_group_struct2_point_reserve(rivetx_sql: &RivetxSql) -> Result<()> {
    for i in 0..20 {
        test_data_create_table(rivetx_sql).await.here()?;
        test_data_clear_table(rivetx_sql).await.here()?;

        let test_data = vec![
            TestData {
                id: 0,
                index: 0,
                key: "abc".into(),
                name_id: 1,
                name_index: 1001,
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
                key: "abc".into(),
                name_id: 2,
                name_index: 1002,
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
                key: "abc".into(),
                name_id: 3,
                name_index: 1003,
                curr_time: chrono::Local::now()
                    .naive_local()
                    .with_nanosecond(0)
                    .unwrap(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 3,
                key: "xyz".into(),
                name_id: 4,
                name_index: 1004,
                curr_time: chrono::Local::now()
                    .naive_local()
                    .with_nanosecond(0)
                    .unwrap(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 4,
                key: "xyz".into(),
                name_id: 5,
                name_index: 1005,
                curr_time: chrono::Local::now()
                    .naive_local()
                    .with_nanosecond(0)
                    .unwrap(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 5,
                key: "xyz".into(),
                name_id: 6,
                name_index: 1006,
                curr_time: chrono::Local::now()
                    .naive_local()
                    .with_nanosecond(0)
                    .unwrap(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 6,
                key: "xyz".into(),
                name_id: 7,
                name_index: 1007,
                curr_time: chrono::Local::now()
                    .naive_local()
                    .with_nanosecond(0)
                    .unwrap(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 7,
                key: "xyz".into(),
                name_id: 8,
                name_index: 1008,
                curr_time: chrono::Local::now()
                    .naive_local()
                    .with_nanosecond(0)
                    .unwrap(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 8,
                key: "xyz".into(),
                name_id: 9,
                name_index: 1009,
                curr_time: chrono::Local::now()
                    .naive_local()
                    .with_nanosecond(0)
                    .unwrap(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 9,
                key: "xyz".into(),
                name_id: 10,
                name_index: 1010,
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
            std::time::Duration::from_secs(10),
        )
        .await
        .here()?;

        if test_data_count_rows(rivetx_sql, "test_data").await != 10 {
            return Err(anyhow!("expected 10 rows"));
        }

        let mut reserve_size = i;

        let res = DeleteBuilder::new("test_data")
            .reserve_size("id", reserve_size, Duration::from_millis(10))
            .exec(rivetx_sql)
            .await
            .here()?;

        if reserve_size > test_data.len() {
            reserve_size = test_data.len();
        }

        let expected_affected = (test_data.len() - reserve_size) as u64;
        if res.total_affected != expected_affected {
            return Err(anyhow!(
                "affected: {} != expected: {}",
                res.total_affected,
                expected_affected
            ));
        }

        let count = test_data_count_rows(rivetx_sql, "test_data").await;
        if count != reserve_size {
            return Err(anyhow!("count: {} != reserve: {}", count, reserve_size));
        }

        let test_data = &test_data[test_data.len() - reserve_size..test_data.len()];
        let rows = test_data_query_all_no_id(rivetx_sql).await.here()?;

        if rows.len() != test_data.len() {
            return Err(anyhow!("rows.len() != testData.len()"));
        }

        for (i, row) in rows.iter().enumerate() {
            if *row != test_data[i] {
                return Err(anyhow!(
                    "row {} mismatch: got {:?}, want {:?}",
                    i,
                    row,
                    test_data[i]
                ));
            }
        }
    }

    log::info!("TestBatchDeletePerGroupStruct2PointReserve test passed  ");
    Ok(())
}
