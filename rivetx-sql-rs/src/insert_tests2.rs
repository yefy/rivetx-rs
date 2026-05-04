use crate::conn::RivetxSql;
use crate::insert::{insert, insert_raw, InsertBuilder};
use crate::util_tests::{
    test_data_count_rows, test_data_create_table, test_data_query_all_no_id,
    test_data_truncate_table, TestData,
};

use anyhow::{anyhow, Result};
use log::info;
use mysql_async::prelude::Queryable;
use mysql_async::Value;
use std::time::Duration;

pub async fn test_insert2(rivetx_sql: &RivetxSql) -> Result<()> {
    test_batch_new_insert_struct_point(rivetx_sql).await?;
    test_batch_new_insert_struct(rivetx_sql).await?;
    test_batch_insert(rivetx_sql).await?;
    test_batch_insert_struct(rivetx_sql).await?;
    test_batch_insert_no_duplicate_update(rivetx_sql).await?;
    test_batch_insert_struct_no_duplicate_update(rivetx_sql).await?;
    Ok(())
}

pub async fn test_batch_insert(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await?;
    test_data_truncate_table(rivetx_sql).await?;

    let cols = vec![
        "index_col".into(),
        "key_col".into(),
        "name_id".into(),
        "name_index".into(),
    ];
    let vals = vec![
        vec![
            Value::from(0),
            Value::from("abc"),
            Value::from(1),
            Value::from(1001),
        ],
        vec![
            Value::from(1),
            Value::from("abc"),
            Value::from(2),
            Value::from(1002),
        ],
        vec![
            Value::from(2),
            Value::from("abc"),
            Value::from(3),
            Value::from(1003),
        ],
        vec![
            Value::from(3),
            Value::from("xyz"),
            Value::from(4),
            Value::from(1004),
        ],
        vec![
            Value::from(4),
            Value::from("xyz"),
            Value::from(5),
            Value::from(1005),
        ],
        vec![
            Value::from(5),
            Value::from("xyz"),
            Value::from(6),
            Value::from(1006),
        ],
        vec![
            Value::from(6),
            Value::from("xyz"),
            Value::from(7),
            Value::from(1007),
        ],
        vec![
            Value::from(7),
            Value::from("xyz"),
            Value::from(8),
            Value::from(1008),
        ],
        vec![
            Value::from(8),
            Value::from("xyz"),
            Value::from(9),
            Value::from(1009),
        ],
        vec![
            Value::from(9),
            Value::from("xyz"),
            Value::from(10),
            Value::from(1010),
        ],
    ];

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
    .await?;

    let count = test_data_count_rows(rivetx_sql, "test_data").await;
    if count != 10 {
        return Err(anyhow!("expected 10 rows, got {}", count));
    }

    let vals_dup = vec![vec![
        Value::from(0),
        Value::from("abc"),
        Value::from(11),
        Value::from(11),
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
    .await?;

    let mut conn = rivetx_sql.get_conn().await?;
    let row_res: Option<(i32, i32)> = conn
        .query_first(
            "SELECT name_id, name_index FROM test_data WHERE index_col = 0 AND key_col = 'abc'",
        )
        .await?;

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

    info!("BatchInsert test passed ✅");
    Ok(())
}

pub async fn test_batch_insert_struct(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await?;
    test_data_truncate_table(rivetx_sql).await?;

    let test_data = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 1,
            name_index: 1001,
        },
        TestData {
            id: 0,
            index: 1,
            key: "abc".into(),
            name_id: 2,
            name_index: 1002,
        },
        TestData {
            id: 0,
            index: 2,
            key: "abc".into(),
            name_id: 3,
            name_index: 1003,
        },
        TestData {
            id: 0,
            index: 3,
            key: "xyz".into(),
            name_id: 4,
            name_index: 1004,
        },
        TestData {
            id: 0,
            index: 4,
            key: "xyz".into(),
            name_id: 5,
            name_index: 1005,
        },
        TestData {
            id: 0,
            index: 5,
            key: "xyz".into(),
            name_id: 6,
            name_index: 1006,
        },
        TestData {
            id: 0,
            index: 6,
            key: "xyz".into(),
            name_id: 7,
            name_index: 1007,
        },
        TestData {
            id: 0,
            index: 7,
            key: "xyz".into(),
            name_id: 8,
            name_index: 1008,
        },
        TestData {
            id: 0,
            index: 8,
            key: "xyz".into(),
            name_id: 9,
            name_index: 1009,
        },
        TestData {
            id: 0,
            index: 9,
            key: "xyz".into(),
            name_id: 10,
            name_index: 1010,
        },
    ];

    let on_duplicate = "name_id = VALUES(name_id), name_index = name_index + VALUES(name_index)";

    insert::<TestData>(
        rivetx_sql,
        &"test_data".into(),
        &test_data,
        2,
        &on_duplicate.into(),
        false,
        Duration::from_secs(10),
    )
    .await?;

    let count = test_data_count_rows(rivetx_sql, "test_data").await;
    if count != 10 {
        return Err(anyhow!("expected 10 rows, got {}", count));
    }

    let data_dup = vec![TestData {
        id: 0,
        index: 0,
        key: "abc".into(),
        name_id: 11,
        name_index: 11,
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
    .await?;

    let mut conn = rivetx_sql.get_conn().await?;
    let row_res: Option<(i32, i32)> = conn
        .query_first(
            "SELECT name_id, name_index FROM test_data WHERE index_col = 0 AND key_col = 'abc'",
        )
        .await?;

    if let Some((name_id, name_index)) = row_res {
        if name_id != 11 || name_index != 1012 {
            return Err(anyhow!(
                "ON DUPLICATE KEY UPDATE failed for struct, got name_id={}, name_index={}",
                name_id,
                name_index
            ));
        }
    }

    info!("BatchInsertStruct test passed ✅");
    Ok(())
}

pub async fn test_batch_insert_no_duplicate_update(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await?;
    test_data_truncate_table(rivetx_sql).await?;

    let cols = vec![
        "index_col".into(),
        "key_col".into(),
        "name_id".into(),
        "name_index".into(),
    ];
    let vals = vec![
        vec![
            Value::from(0),
            Value::from("abc"),
            Value::from(1),
            Value::from(1001),
        ],
        vec![
            Value::from(1),
            Value::from("abc"),
            Value::from(2),
            Value::from(1002),
        ],
        vec![
            Value::from(2),
            Value::from("abc"),
            Value::from(3),
            Value::from(1003),
        ],
        vec![
            Value::from(3),
            Value::from("xyz"),
            Value::from(4),
            Value::from(1004),
        ],
        vec![
            Value::from(4),
            Value::from("xyz"),
            Value::from(5),
            Value::from(1005),
        ],
        vec![
            Value::from(5),
            Value::from("xyz"),
            Value::from(6),
            Value::from(1006),
        ],
        vec![
            Value::from(6),
            Value::from("xyz"),
            Value::from(7),
            Value::from(1007),
        ],
        vec![
            Value::from(7),
            Value::from("xyz"),
            Value::from(8),
            Value::from(1008),
        ],
        vec![
            Value::from(8),
            Value::from("xyz"),
            Value::from(9),
            Value::from(1009),
        ],
        vec![
            Value::from(9),
            Value::from("xyz"),
            Value::from(10),
            Value::from(1010),
        ],
    ];

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
    .await?;

    if test_data_count_rows(rivetx_sql, "test_data").await != 10 {
        return Err(anyhow!("expected 10 rows"));
    }

    info!("BatchInsert without ON DUPLICATE KEY UPDATE passed ✅");
    Ok(())
}

pub async fn test_batch_insert_struct_no_duplicate_update(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await?;
    test_data_truncate_table(rivetx_sql).await?;

    let test_data = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 1,
            name_index: 1001,
        },
        TestData {
            id: 0,
            index: 1,
            key: "abc".into(),
            name_id: 2,
            name_index: 1002,
        },
        TestData {
            id: 0,
            index: 2,
            key: "abc".into(),
            name_id: 3,
            name_index: 1003,
        },
        TestData {
            id: 0,
            index: 3,
            key: "xyz".into(),
            name_id: 4,
            name_index: 1004,
        },
        TestData {
            id: 0,
            index: 4,
            key: "xyz".into(),
            name_id: 5,
            name_index: 1005,
        },
        TestData {
            id: 0,
            index: 5,
            key: "xyz".into(),
            name_id: 6,
            name_index: 1006,
        },
        TestData {
            id: 0,
            index: 6,
            key: "xyz".into(),
            name_id: 7,
            name_index: 1007,
        },
        TestData {
            id: 0,
            index: 7,
            key: "xyz".into(),
            name_id: 8,
            name_index: 1008,
        },
        TestData {
            id: 0,
            index: 8,
            key: "xyz".into(),
            name_id: 9,
            name_index: 1009,
        },
        TestData {
            id: 0,
            index: 9,
            key: "xyz".into(),
            name_id: 10,
            name_index: 1010,
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
    .await?;

    if test_data_count_rows(rivetx_sql, "test_data").await != 10 {
        return Err(anyhow!("expected 10 rows"));
    }

    info!("BatchInsertStruct without ON DUPLICATE KEY UPDATE passed ✅");
    Ok(())
}

pub async fn test_batch_new_insert_struct(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await?;
    test_data_truncate_table(rivetx_sql).await?;

    let test_data = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 1,
            name_index: 1001,
        },
        TestData {
            id: 0,
            index: 1,
            key: "abc".into(),
            name_id: 2,
            name_index: 1002,
        },
        TestData {
            id: 0,
            index: 2,
            key: "abc".into(),
            name_id: 3,
            name_index: 1003,
        },
        TestData {
            id: 0,
            index: 3,
            key: "xyz".into(),
            name_id: 4,
            name_index: 1004,
        },
        TestData {
            id: 0,
            index: 4,
            key: "xyz".into(),
            name_id: 5,
            name_index: 1005,
        },
        TestData {
            id: 0,
            index: 5,
            key: "xyz".into(),
            name_id: 6,
            name_index: 1006,
        },
        TestData {
            id: 0,
            index: 6,
            key: "xyz".into(),
            name_id: 7,
            name_index: 1007,
        },
        TestData {
            id: 0,
            index: 7,
            key: "xyz".into(),
            name_id: 8,
            name_index: 1008,
        },
        TestData {
            id: 0,
            index: 8,
            key: "xyz".into(),
            name_id: 9,
            name_index: 1009,
        },
        TestData {
            id: 0,
            index: 9,
            key: "xyz".into(),
            name_id: 10,
            name_index: 1010,
        },
    ];

    let on_duplicate = "name_id = VALUES(name_id), name_index = name_index + VALUES(name_index)";
    let result = InsertBuilder::new(rivetx_sql, "test_data", test_data.clone())
        .batch_size(2)
        .on_duplicate_update(on_duplicate)
        .timeout(Duration::from_secs(10))
        .exec()
        .await?;

    info!("BatchInsertStruct test result:{:?}", result);

    let mut conn = rivetx_sql.get_conn().await?;
    let id: u64 = conn
        .query_first("SELECT id FROM test_data order by id DESC limit 1")
        .await?
        .unwrap_or(0);

    if result.last_insert_id != id {
        return Err(anyhow!(
            "result.last_insert_id:{} != id:{} ",
            result.last_insert_id,
            id
        ));
    }

    let count = test_data_count_rows(rivetx_sql, "test_data").await;
    if count != 10 || count != result.total_affected as usize {
        return Err(anyhow!(
            "expected 10 rows, got {}|{}",
            count,
            result.total_affected
        ));
    }

    let rows = test_data_query_all_no_id(rivetx_sql).await?;
    if rows.len() != test_data.len() {
        return Err(anyhow!("len(rows) != len(testData)"));
    }

    for (i, row) in rows.iter().enumerate() {
        if row.index != test_data[i].index
            || row.key != test_data[i].key
            || row.name_id != test_data[i].name_id
            || row.name_index != test_data[i].name_index
        {
            return Err(anyhow!("row {} mismatch", i));
        }
    }

    let data_dup = vec![TestData {
        id: 0,
        index: 0,
        key: "abc".into(),
        name_id: 11,
        name_index: 11,
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
    .await?;

    let row_res: Option<(i32, i32)> = conn
        .query_first(
            "SELECT name_id, name_index FROM test_data WHERE index_col = 0 AND key_col = 'abc'",
        )
        .await?;
    if let Some((name_id, name_index)) = row_res {
        if name_id != 11 || name_index != 1012 {
            return Err(anyhow!("ON DUPLICATE KEY UPDATE failed for struct"));
        }
    }

    info!("BatchInsertStruct test passed ✅");
    Ok(())
}

pub async fn test_batch_new_insert_struct_point(rivetx_sql: &RivetxSql) -> Result<()> {
    test_data_create_table(rivetx_sql).await?;
    test_data_truncate_table(rivetx_sql).await?;

    let test_data = vec![
        TestData {
            id: 0,
            index: 0,
            key: "abc".into(),
            name_id: 1,
            name_index: 1001,
        },
        TestData {
            id: 0,
            index: 1,
            key: "abc".into(),
            name_id: 2,
            name_index: 1002,
        },
        TestData {
            id: 0,
            index: 2,
            key: "abc".into(),
            name_id: 3,
            name_index: 1003,
        },
        TestData {
            id: 0,
            index: 3,
            key: "xyz".into(),
            name_id: 4,
            name_index: 1004,
        },
        TestData {
            id: 0,
            index: 4,
            key: "xyz".into(),
            name_id: 5,
            name_index: 1005,
        },
        TestData {
            id: 0,
            index: 5,
            key: "xyz".into(),
            name_id: 6,
            name_index: 1006,
        },
        TestData {
            id: 0,
            index: 6,
            key: "xyz".into(),
            name_id: 7,
            name_index: 1007,
        },
        TestData {
            id: 0,
            index: 7,
            key: "xyz".into(),
            name_id: 8,
            name_index: 1008,
        },
        TestData {
            id: 0,
            index: 8,
            key: "xyz".into(),
            name_id: 9,
            name_index: 1009,
        },
        TestData {
            id: 0,
            index: 9,
            key: "xyz".into(),
            name_id: 10,
            name_index: 1010,
        },
    ];

    let on_duplicate = "name_id = VALUES(name_id), name_index = name_index + VALUES(name_index)";
    let result = InsertBuilder::new(rivetx_sql, "test_data", test_data.clone())
        .batch_size(2)
        .on_duplicate_update(on_duplicate)
        .timeout(Duration::from_secs(10))
        .exec()
        .await?;

    info!("BatchInsertStruct test result:{:?}", result);

    let mut conn = rivetx_sql.get_conn().await?;
    let id: u64 = conn
        .query_first("SELECT id FROM test_data order by id DESC limit 1")
        .await?
        .unwrap_or(0);

    if result.last_insert_id != id {
        return Err(anyhow!("result.last_insert_id mismatch"));
    }

    if test_data_count_rows(rivetx_sql, "test_data").await != 10 {
        return Err(anyhow!("count mismatch"));
    }

    let rows = test_data_query_all_no_id(rivetx_sql).await?;
    for (i, row) in rows.iter().enumerate() {
        if row.index != test_data[i].index {
            return Err(anyhow!("row {} mismatch", i));
        }
    }

    info!("BatchInsertStruct test passed ✅");
    Ok(())
}
