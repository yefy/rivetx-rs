use crate::conn::RivetxSql;
use crate::insert::insert;
use crate::select::{select_raw, SelectBuilder};
use crate::util::QueryCond;
use crate::util_tests::{
    test_data_clear_table, test_data_count_rows, test_data_create_table, test_data_drop_table,
    test_data_query_all, test_data_query_all_by_id, zero_naive_date_time, TestData, TestDataByAs,
    TestDataByD, TestDataNoExport,
};
use anyhow::Context;

use crate::util_tests::Testkey;
use anyhow::{anyhow, Result};
use chrono::Timelike;
use log::info;
use std::time::Duration;

pub async fn test_select(rivetx_sql: &RivetxSql) -> Result<()> {
    test_select_with_where_point(rivetx_sql).await.here()?;
    test_select_with_where(rivetx_sql).await.here()?;
    test_select_with_where_join(rivetx_sql).await.here()?;
    Ok(())
}

pub async fn test_select_with_where_point(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_drop_table(rivetx_sql).await.here()?;
    test_data_create_table(rivetx_sql).await.here()?;
    test_data_clear_table(rivetx_sql).await.here()?;

    if test_data_count_rows(rivetx_sql, "test_data").await != 0 {
        return Err(anyhow!("countRows(db, \"test_data\") != 0"));
    }

    let test_data = vec![
        TestData {
            id: 1,
            index: 1,
            key: "hex".into(),
            name_id: 100,
            name_index: 1000,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 2,
            index: 1,
            key: "abc".into(),
            name_id: 101,
            name_index: 1001,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 3,
            index: 1,
            key: "def".into(),
            name_id: 102,
            name_index: 1002,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 4,
            index: 2,
            key: "ghi".into(),
            name_id: 103,
            name_index: 1003,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 5,
            index: 2,
            key: "xyz".into(),
            name_id: 104,
            name_index: 1004,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 6,
            index: 2,
            key: "kyl".into(),
            name_id: 105,
            name_index: 1005,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
    ];

    insert::<TestData>(
        rivetx_sql,
        &"test_data".into(),
        &test_data,
        2,
        &"".into(),
        false,
        Duration::from_secs(10),
    )
    .await
    .here()?;

    if true {
        use mysql_async::prelude::Queryable;
        let mut conn = rivetx_sql.get_conn().await.here()?;
        let row: mysql_async::Row = conn
            .query_first("SELECT id FROM test_data LIMIT 1")
            .await?
            .unwrap();
        log::info!("Raw protocol ID: {:?}", row);
    }

    if true {
        use mysql_async::prelude::Queryable;
        let mut conn = rivetx_sql.get_conn().await.here()?;
        let row: mysql_async::Value = conn
            .query_first("SELECT id FROM test_data LIMIT 1")
            .await?
            .unwrap();
        log::info!("Value protocol ID: {:?}", row);
    }

    if true {
        use mysql_async::prelude::Queryable;
        let mut conn = rivetx_sql.get_conn().await.here()?;
        let res: Vec<(u64, i64, String, i64, i64)> = conn
            .exec(
                "SELECT id, index_col, key_col, name_id, name_index FROM test_data",
                (),
            )
            .await
            .here()?;
        log::info!("res {:?}", res);
    }

    {
        let res1: Vec<TestData> = SelectBuilder::new("test_data")
            .where_eq("1", 1)
            .order("order by index_col, key_col")
            .exec(rivetx_sql)
            .await
            .here()?;

        info!("res1:{:?}", res1);
        for res in &res1 {
            info!("res:{:?}", res);
        }

        if res1.len() != test_data.len() {
            return Err(anyhow!("len(res1) != len(test_data)"));
        }

        let rows = test_data_query_all(rivetx_sql).await.here()?;
        for (i, row) in rows.iter().enumerate() {
            if row.id != res1[i].id || row.index != res1[i].index || row.key != res1[i].key {
                return Err(anyhow!("row != *res1[i] at index {}", i));
            }
        }
    }

    for i in 0..20 {
        let mut limit = i;
        info!("OrderFieldSelect true index:{}", i);
        let res1: Vec<TestData> = SelectBuilder::new("test_data")
            .order_field_select("id", true, i)
            .timeout(Duration::from_secs(10))
            .exec(rivetx_sql)
            .await
            .here()?;

        info!("res1:{:?}", res1);
        for res in &res1 {
            info!("res:{:?}", res);
        }

        if limit > test_data.len() {
            limit = test_data.len();
        }

        if res1.len() != limit {
            return Err(anyhow!("expected {} rows, got {}", limit, res1.len()));
        }

        let rows = test_data_query_all_by_id(rivetx_sql, true, i)
            .await
            .here()?;
        for (index, row) in rows.iter().enumerate() {
            info!("index:{}, res:{:?}, row:{:?}", index, res1[index], row);
            if row.id != res1[index].id {
                return Err(anyhow!("row.Id != res1[index].Id"));
            }
        }
    }

    for i in 0..20 {
        let mut limit = i;
        info!("OrderFieldSelect false index:{}", i);
        let res1: Vec<TestData> = SelectBuilder::new("test_data")
            .order_field_select("id", false, i)
            .timeout(Duration::from_secs(10))
            .exec(rivetx_sql)
            .await
            .here()?;

        info!("res1:{:?}", res1);
        for res in &res1 {
            info!("res:{:?}", res);
        }

        if limit > test_data.len() {
            limit = test_data.len();
        }

        if res1.len() != limit {
            return Err(anyhow!("expected {} rows, got {}", limit, res1.len()));
        }

        let rows = test_data_query_all_by_id(rivetx_sql, false, i)
            .await
            .here()?;
        for (index, row) in rows.iter().enumerate() {
            info!("index:{}, res:{:?}, row:{:?}", index, res1[index], row);
            if row.id != res1[index].id {
                return Err(anyhow!("row.Id != res1[index].Id"));
            }
        }
    }

    {
        let res1: Vec<TestDataNoExport> = SelectBuilder::new("test_data")
            .where_eq("index_col", 1)
            .timeout(Duration::from_secs(10))
            .exec(rivetx_sql)
            .await
            .here()?;

        info!("res1:{:?}", res1);
        for res in &res1 {
            info!("res:{:?}", res);
        }
        if res1.len() != 3 {
            return Err(anyhow!("expected 3 rows, got {}", res1.len()));
        }
    }

    {
        info!("test WhereIn");
        let in_vals = vec![vec!["yyy".into()], vec!["xxx".into()]];
        let res1: Vec<TestData> = SelectBuilder::new("test_data")
            .where_eq("index_col", 1)
            .where_in(vec!["key_col".into()], in_vals)
            .timeout(Duration::from_secs(10))
            .exec(rivetx_sql)
            .await
            .here()?;

        info!("res1:{:?}", res1);
        for res in &res1 {
            info!("res:{:?}", res);
        }
        if res1.len() != 0 {
            return Err(anyhow!("expected 0 rows, got {}", res1.len()));
        }
    }

    {
        let res = SelectBuilder::<TestData>::new("test_data")
            .where_eq("index_col", 1)
            .where_in(vec!["key_col".into()], vec![])
            .timeout(Duration::from_secs(10))
            .exec(rivetx_sql)
            .await;

        if res.is_err() {
            info!("err:{:?}", res.err());
        } else {
            return Err(anyhow!("expected error for empty WhereIn, but got success"));
        }
    }

    {
        let mut cond = QueryCond::new();
        cond.fixed_cols.push("index_col".into());
        cond.fixed_vals.push(1.into());

        let res1: Vec<TestData> = select_raw(
            rivetx_sql,
            &"test_data".into(),
            &"".into(),
            &cond,
            &"".into(),
            &vec![],
            &"".into(),
            0,
            0,
            0,
            Duration::from_secs(10),
        )
        .await
        .here()?;

        info!("res1:{:?}", res1);
        for res in &res1 {
            info!("res:{:?}", res);
        }
        if res1.len() != 3 {
            return Err(anyhow!("expected 3 rows, got {}", res1.len()));
        }
    }

    {
        let res1: Vec<TestData> = SelectBuilder::new("test_data")
            .where_eq("index_col", 1)
            .timeout(Duration::from_secs(10))
            .exec(rivetx_sql)
            .await
            .here()?;
        info!("res1:{:?}", res1);
        if res1.len() != 3 {
            return Err(anyhow!("expected 3 rows, got {}", res1.len()));
        }
    }

    {
        let res1: Vec<TestData> = SelectBuilder::new("test_data")
            .where_cond("index_col = ?", vec![1.into()])
            .where_cond("and key_col = ?", vec!["abc".into()])
            .timeout(Duration::from_secs(10))
            .exec(rivetx_sql)
            .await
            .here()?;

        info!("res1:{:?}", res1);
        if res1.len() != 1 {
            return Err(anyhow!("expected 1 rows, got {}", res1.len()));
        }
    }

    {
        let res1: Vec<TestData> = SelectBuilder::new("test_data")
            .where_cond(
                "index_col = ? and key_col = ?",
                vec![1.into(), "abc".into()],
            )
            .timeout(Duration::from_secs(10))
            .exec(rivetx_sql)
            .await
            .here()?;

        info!("res1:{:?}", res1);
        if res1.len() != 1 {
            return Err(anyhow!("expected 1 rows, got {}", res1.len()));
        }
    }

    Ok(())
}
pub async fn test_select_with_where(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await.here()?;
    test_data_clear_table(rivetx_sql).await.here()?;

    let test_data = vec![
        TestData {
            id: 1,
            index: 1,
            key: "hex".into(),
            name_id: 100,
            name_index: 1000,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 2,
            index: 1,
            key: "abc".into(),
            name_id: 101,
            name_index: 1001,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 3,
            index: 1,
            key: "def".into(),
            name_id: 102,
            name_index: 1002,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 4,
            index: 2,
            key: "ghi".into(),
            name_id: 103,
            name_index: 1003,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 5,
            index: 2,
            key: "xyz".into(),
            name_id: 104,
            name_index: 1004,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 6,
            index: 2,
            key: "kyl".into(),
            name_id: 105,
            name_index: 1005,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
    ];

    insert::<TestData>(
        rivetx_sql,
        &"test_data".into(),
        &test_data,
        2,
        &"".into(),
        false,
        Duration::from_secs(10),
    )
    .await
    .here()?;

    {
        let mut cond = QueryCond::new();
        cond.fixed_cols.push("index_col".into());
        cond.fixed_vals.push(1.into());

        let res1: Vec<TestData> = select_raw(
            rivetx_sql,
            &"test_data".into(),
            &"".into(),
            &cond,
            &"".into(),
            &vec![],
            &"".into(),
            0,
            0,
            0,
            Duration::from_secs(10),
        )
        .await
        .here()?;

        info!("res1:{:?}", res1);
        for res in &res1 {
            info!("res:{:?}", res);
        }
        if res1.len() != 3 {
            return Err(anyhow!("expected 3 rows, got {}", res1.len()));
        }
    }

    {
        let mut cond = QueryCond::new();
        cond.fixed_cols.push("index_col".into());
        cond.fixed_vals.push(2.into());
        cond.in_cols.push("key_col".into());
        cond.in_vals.push(vec!["ghi".into()]);
        cond.in_vals.push(vec!["xyz".into()]);
        cond.in_vals.push(vec!["kyl".into()]);

        let res2: Vec<TestData> = select_raw(
            rivetx_sql,
            &"test_data".into(),
            &"".into(),
            &cond,
            &"".into(),
            &vec![],
            &"".into(),
            0,
            0,
            0,
            Duration::from_secs(10),
        )
        .await
        .here()?;

        info!("res2:{:?}", res2);
        for res in &res2 {
            info!("res:{:?}", res);
        }
        if res2.len() != 3 {
            return Err(anyhow!("expected 3 rows, got {}", res2.len()));
        }
    }

    {
        let res4: Vec<TestData> = select_raw(
            rivetx_sql,
            &"test_data".into(),
            &"".into(),
            &QueryCond::new(),
            &"1=1".into(),
            &vec![],
            &"".into(),
            0,
            0,
            0,
            Duration::from_secs(10),
        )
        .await
        .here()?;

        info!("res4:{:?}", res4);
        for res in &res4 {
            info!("res:{:?}", res);
        }
        if res4.len() != 6 {
            return Err(anyhow!("expected 6 rows, got {}", res4.len()));
        }
    }

    info!("  rivetx-sql select tests passed");
    Ok(())
}

pub async fn test_select_with_where_join(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await.here()?;
    test_data_clear_table(rivetx_sql).await.here()?;

    let test_data = vec![
        TestData {
            id: 1,
            index: 1,
            key: "hex".into(),
            name_id: 100,
            name_index: 1000,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 2,
            index: 1,
            key: "abc".into(),
            name_id: 101,
            name_index: 1001,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 3,
            index: 1,
            key: "def".into(),
            name_id: 102,
            name_index: 1002,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 4,
            index: 2,
            key: "ghi".into(),
            name_id: 103,
            name_index: 1003,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 5,
            index: 2,
            key: "xyz".into(),
            name_id: 104,
            name_index: 1004,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        TestData {
            id: 6,
            index: 2,
            key: "kyl".into(),
            name_id: 105,
            name_index: 1005,
            curr_time: chrono::Local::now()
                .naive_local()
                .with_nanosecond(0)
                .unwrap(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
    ];
    insert::<TestData>(
        rivetx_sql,
        &"test_data".into(),
        &test_data,
        2,
        &"".into(),
        false,
        Duration::from_secs(10),
    )
    .await
    .here()?;

    let test_keys = vec![
        Testkey {
            id: 0,
            index: 1,
            key: "hex".into(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        Testkey {
            id: 0,
            index: 1,
            key: "abc".into(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        Testkey {
            id: 0,
            index: 1,
            key: "def".into(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        Testkey {
            id: 0,
            index: 2,
            key: "ghi".into(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        Testkey {
            id: 0,
            index: 2,
            key: "xyz".into(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
        Testkey {
            id: 0,
            index: 2,
            key: "kyl".into(),
            created_at: zero_naive_date_time(),
            updated_at: zero_naive_date_time(),
        },
    ];
    insert::<Testkey>(
        rivetx_sql,
        &"test_key".into(),
        &test_keys,
        2,
        &"".into(),
        false,
        Duration::from_secs(10),
    )
    .await
    .here()?;

    let join_str = "JOIN test_key k ON d.index_col = k.index_col AND d.key_col = k.key_col";

    {
        let mut cond1 = QueryCond::new();
        cond1.fixed_cols.push("d.index_col".into());
        cond1.fixed_vals.push(1.into());

        let res1: Vec<TestDataByD> = select_raw(
            rivetx_sql,
            &"test_data d".into(),
            &join_str.into(),
            &cond1,
            &"".into(),
            &vec![],
            &"".into(),
            0,
            0,
            0,
            Duration::from_secs(10),
        )
        .await
        .here()?;

        info!("res1:{:?}", res1);
        for res in &res1 {
            info!("res:{:?}", res);
        }
        if res1.len() != 3 {
            return Err(anyhow!("expected 3 rows, got {}", res1.len()));
        }
    }

    {
        let mut cond_as = QueryCond::new();
        cond_as.fixed_cols.push("d.index_col".into());
        cond_as.fixed_vals.push(1.into());

        let res_as: Vec<TestDataByAs> = select_raw(
            rivetx_sql,
            &"test_data d".into(),
            &join_str.into(),
            &cond_as,
            &"".into(),
            &vec![],
            &"".into(),
            0,
            0,
            0,
            Duration::from_secs(10),
        )
        .await
        .here()?;

        info!("res_as:{:?}", res_as);
        for res in &res_as {
            info!("res:{:?}", res);
        }
        if res_as.len() != 3 {
            return Err(anyhow!("expected 3 rows, got {}", res_as.len()));
        }
    }

    {
        let mut cond2 = QueryCond::new();
        cond2.fixed_cols.push("d.index_col".into());
        cond2.fixed_vals.push(2.into());
        cond2.in_cols.push("d.key_col".into());
        cond2.in_vals.push(vec!["ghi".into()]);
        cond2.in_vals.push(vec!["xyz".into()]);
        cond2.in_vals.push(vec!["kyl".into()]);

        let res2: Vec<TestDataByD> = select_raw(
            rivetx_sql,
            &"test_data d".into(),
            &join_str.into(),
            &cond2,
            &"".into(),
            &vec![],
            &"".into(),
            0,
            0,
            0,
            Duration::from_secs(10),
        )
        .await
        .here()?;

        info!("res2:{:?}", res2);
        for res in &res2 {
            info!("res:{:?}", res);
        }
        if res2.len() != 3 {
            return Err(anyhow!("expected 3 rows, got {}", res2.len()));
        }
    }

    {
        let res4: Vec<TestDataByD> = select_raw(
            rivetx_sql,
            &"test_data d".into(),
            &join_str.into(),
            &QueryCond::new(),
            &"1=1".into(),
            &vec![],
            &"".into(),
            0,
            0,
            0,
            Duration::from_secs(10),
        )
        .await
        .here()?;

        info!("res4:{:?}", res4);
        for res in &res4 {
            info!("res:{:?}", res);
        }
        if res4.len() != 6 {
            return Err(anyhow!("expected 6 rows, got {}", res4.len()));
        }
    }

    info!("  rivetx-sql select join tests passed");
    Ok(())
}
