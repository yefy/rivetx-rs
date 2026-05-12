#[cfg(test)]
mod tests {
    use crate::insert::insert;
    use crate::update::{UpdateBuilder, UpdateResult};
    use crate::util::{BATCH_SIZE, TIMEOUT};
    use crate::util_tests::{
        test_data_clear_table, test_data_create_table, test_data_query_all_no_id,
        zero_naive_date_time, TestData,
    };
    use anyhow::Result;
    use chrono::Timelike;
    use mysql_async::Value;
    use rivetx_core::rivetx_string::RivetxString;
    use std::time::Duration;

    // ────────── Helper: build UPDATE SQL similar to update_raw ──────────

    /// Build an UPDATE ... JOIN (VALUES ...) SQL string and its arguments,
    /// mirroring the logic in update_raw.
    fn build_update_sql(
        table: &str,
        cols: &[RivetxString],
        vals: &[Vec<Value>],
        join_on: &[RivetxString],
        set_expr: &[RivetxString],
        max_per_batch: usize,
    ) -> (String, Vec<Value>) {
        let effective_batch_size = if max_per_batch == 0 {
            BATCH_SIZE
        } else {
            max_per_batch
        };

        let mut rows_sql = Vec::with_capacity(effective_batch_size.min(vals.len()) * 4);
        let mut args = Vec::with_capacity(vals.len() * cols.len());

        for v in vals.iter().take(effective_batch_size.min(vals.len())) {
            let placeholders = vec!["?"; v.len()].join(",");
            rows_sql.push(format!("ROW({})", placeholders));
            args.extend(v.iter().cloned());
        }

        let on_conditions: Vec<String> = join_on
            .iter()
            .map(|c| format!("u.{} = v.{}", c, c))
            .collect();

        let query = format!(
            "UPDATE {} AS u JOIN (VALUES {}) AS v({}) ON {} SET {}",
            table,
            rows_sql.join(", "),
            cols.join(", "),
            on_conditions.join(" AND "),
            set_expr.join(", ")
        );

        (query, args)
    }

    // ────────── SQL Building Tests ──────────

    #[test]
    fn test_update_build_basic() {
        let cols: Vec<RivetxString> = vec![
            "index_col".into(),
            "key_col".into(),
            "name_id".into(),
            "name_index".into(),
        ];
        let vals = vec![
            vec![
                Value::from(0i32),
                Value::from("abc"),
                Value::from(10i32),
                Value::from(10i32),
            ],
            vec![
                Value::from(1i32),
                Value::from("xyz"),
                Value::from(20i32),
                Value::from(20i32),
            ],
        ];
        let join_on: Vec<RivetxString> =
            vec!["index_col".into(), "key_col".into()];
        let set_expr: Vec<RivetxString> = vec![
            "u.name_id = v.name_id".into(),
            "u.name_index = u.name_index + v.name_index".into(),
        ];

        let (sql, args) = build_update_sql("test_data", &cols, &vals, &join_on, &set_expr, 0);

        assert!(sql.starts_with("UPDATE test_data AS u JOIN (VALUES"));
        assert!(sql.contains("AS v(index_col, key_col, name_id, name_index)"));
        assert!(sql.contains("ON u.index_col = v.index_col AND u.key_col = v.key_col"));
        assert!(sql.contains("SET u.name_id = v.name_id, u.name_index = u.name_index + v.name_index"));
        assert_eq!(args.len(), 8); // 2 rows * 4 cols
    }

    #[test]
    fn test_update_build_single_row() {
        let cols: Vec<RivetxString> = vec!["id".into(), "name_id".into()];
        let vals = vec![vec![Value::from(1u64), Value::from(100i32)]];
        let join_on: Vec<RivetxString> = vec!["id".into()];
        let set_expr: Vec<RivetxString> = vec!["u.name_id = v.name_id".into()];

        let (sql, args) = build_update_sql("test_table", &cols, &vals, &join_on, &set_expr, 0);

        assert!(sql.contains("UPDATE test_table AS u JOIN (VALUES"));
        assert!(sql.contains("AS v(id, name_id)"));
        assert!(sql.contains("ON u.id = v.id"));
        assert!(sql.contains("SET u.name_id = v.name_id"));
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_update_build_multiple_set_expr() {
        let cols: Vec<RivetxString> = vec!["id".into(), "name_id".into(), "name_index".into()];
        let vals = vec![vec![
            Value::from(1u64),
            Value::from(10i32),
            Value::from(100i32),
        ]];
        let join_on: Vec<RivetxString> = vec!["id".into()];
        let set_expr: Vec<RivetxString> = vec![
            "u.name_id = v.name_id".into(),
            "u.name_index = u.name_index + v.name_index".into(),
            "u.updated_at = NOW()".into(),
        ];

        let (sql, _args) = build_update_sql("test_data", &cols, &vals, &join_on, &set_expr, 0);

        assert!(sql.contains("SET u.name_id = v.name_id, u.name_index = u.name_index + v.name_index, u.updated_at = NOW()"));
    }

    #[test]
    fn test_update_build_with_custom_batch_size() {
        let cols: Vec<RivetxString> = vec!["id".into()];
        let vals: Vec<Vec<Value>> = (0..10)
            .map(|i| vec![Value::from(i as u64)])
            .collect();
        let join_on: Vec<RivetxString> = vec!["id".into()];
        let set_expr: Vec<RivetxString> = vec!["u.name_id = v.name_id".into()];

        // batch_size=3, so only 3 rows should appear in the VALUES clause
        let (sql, args) = build_update_sql("test_data", &cols, &vals, &join_on, &set_expr, 3);

        let row_count = sql.matches("ROW(").count();
        assert_eq!(row_count, 3, "Expected 3 ROW(...) entries with batch_size=3");
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn test_update_build_single_join_on() {
        let cols: Vec<RivetxString> = vec!["id".into(), "name_id".into()];
        let vals = vec![vec![Value::from(1u64), Value::from(100i32)]];
        let join_on: Vec<RivetxString> = vec!["id".into()];
        let set_expr: Vec<RivetxString> = vec!["u.name_id = v.name_id".into()];

        let (sql, _args) = build_update_sql("test_data", &cols, &vals, &join_on, &set_expr, 0);

        assert!(sql.contains("ON u.id = v.id"));
        assert!(!sql.contains("AND"));
    }

    #[test]
    fn test_update_build_multiple_join_on() {
        let cols: Vec<RivetxString> = vec!["index_col".into(), "key_col".into(), "name_id".into()];
        let vals = vec![vec![
            Value::from(0i32),
            Value::from("abc"),
            Value::from(10i32),
        ]];
        let join_on: Vec<RivetxString> =
            vec!["index_col".into(), "key_col".into(), "name_id".into()];
        let set_expr: Vec<RivetxString> = vec!["u.name_index = v.name_index".into()];

        let (sql, _args) =
            build_update_sql("test_data", &cols, &vals, &join_on, &set_expr, 0);

        assert!(sql.contains("ON u.index_col = v.index_col AND u.key_col = v.key_col AND u.name_id = v.name_id"));
    }

    // ────────── UpdateResult Tests ──────────

    #[test]
    fn test_update_result_default() {
        let result = UpdateResult::default();

        assert_eq!(result.total_affected, 0);
        assert_eq!(result.last_insert_id, 0);
    }

    #[test]
    fn test_update_result_creation() {
        let result = UpdateResult {
            total_affected: 42,
            last_insert_id: 100,
        };

        assert_eq!(result.total_affected, 42);
        assert_eq!(result.last_insert_id, 100);
    }

    #[test]
    fn test_update_result_zero_affected() {
        let result = UpdateResult {
            total_affected: 0,
            last_insert_id: 0,
        };

        assert_eq!(result.total_affected, 0);
        assert_eq!(result.last_insert_id, 0);
    }

    #[test]
    fn test_update_result_large_values() {
        let result = UpdateResult {
            total_affected: 999999,
            last_insert_id: 888888,
        };

        assert_eq!(result.total_affected, 999999);
        assert_eq!(result.last_insert_id, 888888);
    }

    // ────────── UpdateBuilder Integration Tests (with real DB) ──────────

    /// Helper to create a mock RivetxSql for builder construction.
    fn make_mock_sql() -> crate::conn::RivetxSql {
        crate::conn::RivetxSql::new(
            "mysql://root:Yfygz@389@192.168.192.139:3306/test_db",
            1,
            1,
        )
        .expect("Failed to create mock RivetxSql")
    }

    /// Helper: insert initial test data for update tests
    async fn insert_initial_data(
        rivetx_sql: &crate::conn::RivetxSql,
        curr_time: chrono::NaiveDateTime,
    ) -> Result<Vec<TestData>> {
        let test_data = vec![
            TestData {
                id: 0,
                index: 0,
                key: "abc".into(),
                name_id: 1,
                name_index: 1000,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 1,
                key: "xyz".into(),
                name_id: 2,
                name_index: 2000,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 2,
                key: "def".into(),
                name_id: 3,
                name_index: 3000,
                curr_time: curr_time.clone(),
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

        Ok(test_data)
    }

    /// Helper: verify update results match expected values
    fn verify_update_results(
        rows: &[TestData],
        expected: &[TestData],
    ) -> Result<()> {
        if rows.len() != expected.len() {
            anyhow::bail!(
                "len(rows) {} != len(expected) {}",
                rows.len(),
                expected.len()
            );
        }
        for (i, row) in rows.iter().enumerate() {
            if row != &expected[i] {
                anyhow::bail!(
                    "row {} mismatch: got {:?}, want {:?}",
                    i,
                    row,
                    expected[i]
                );
            }
        }
        Ok(())
    }

    /// Helper: create table and clear data for a test
    async fn setup_table(rivetx_sql: &crate::conn::RivetxSql) {
        test_data_create_table(rivetx_sql).await.unwrap();
        test_data_clear_table(rivetx_sql).await.unwrap();
    }

    /// Helper: get current time truncated to seconds
    fn now_truncated() -> chrono::NaiveDateTime {
        chrono::Local::now()
            .naive_local()
            .with_nanosecond(0)
            .unwrap()
    }

    /// Single entry point for all UpdateBuilder integration tests.
    /// Runs sequentially to avoid DB race conditions.
    #[tokio::test]
    async fn test_update_builder_integration() {
        let rivetx_sql = make_mock_sql();

        // Run all integration tests sequentially
        test_exec_basic(&rivetx_sql).await;
        test_exec_with_expression(&rivetx_sql).await;
        test_exec_with_batch_size(&rivetx_sql).await;
        test_exec_with_timeout(&rivetx_sql).await;
        test_exec_chained_calls(&rivetx_sql).await;
        test_exec_empty_data(&rivetx_sql).await;
        test_exec_empty_join_on(&rivetx_sql).await;
        test_exec_empty_set_expr(&rivetx_sql).await;
        test_exec_zero_batch_size(&rivetx_sql).await;
        test_exec_zero_timeout(&rivetx_sql).await;
        test_exec_multiple_data_items(&rivetx_sql).await;
    }

    async fn test_exec_basic(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        let updates: Vec<TestData> = initial_data
            .iter()
            .map(|d| TestData {
                id: 0,
                index: d.index,
                key: d.key.clone(),
                name_id: d.name_id * 10,
                name_index: d.name_index / 100,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            })
            .collect();

        let join_on = vec!["index_col".into(), "key_col".into()];
        let set_expr = vec![
            "u.name_id = v.name_id".into(),
            "u.name_index = v.name_index".into(),
        ];

        let result = UpdateBuilder::new(rivetx_sql, "test_data", updates)
            .join_on(join_on)
            .set_expr(set_expr)
            .exec()
            .await
            .unwrap();

        assert_eq!(result.total_affected, 3);

        let rows = test_data_query_all_no_id(rivetx_sql).await.unwrap();
        let expected = vec![
            TestData {
                id: 0,
                index: 0,
                key: "abc".into(),
                name_id: 10,
                name_index: 10,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 1,
                key: "xyz".into(),
                name_id: 20,
                name_index: 20,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 2,
                key: "def".into(),
                name_id: 30,
                name_index: 30,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
        ];

        verify_update_results(&rows, &expected).unwrap();
    }

    async fn test_exec_with_expression(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        let updates: Vec<TestData> = initial_data
            .iter()
            .map(|d| TestData {
                id: 0,
                index: d.index,
                key: d.key.clone(),
                name_id: d.name_id * 10,
                name_index: d.name_index / 100,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            })
            .collect();

        let join_on = vec!["index_col".into(), "key_col".into()];
        // Use expression: name_index = name_index + v.name_index
        let set_expr = vec![
            "u.name_id = v.name_id".into(),
            "u.name_index = u.name_index + v.name_index".into(),
        ];

        let result = UpdateBuilder::new(rivetx_sql, "test_data", updates)
            .join_on(join_on)
            .set_expr(set_expr)
            .exec()
            .await
            .unwrap();

        assert_eq!(result.total_affected, 3);

        let rows = test_data_query_all_no_id(rivetx_sql).await.unwrap();
        let expected = vec![
            TestData {
                id: 0,
                index: 0,
                key: "abc".into(),
                name_id: 10,
                name_index: 1010, // 1000 + 10
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 1,
                key: "xyz".into(),
                name_id: 20,
                name_index: 2020, // 2000 + 20
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 2,
                key: "def".into(),
                name_id: 30,
                name_index: 3030, // 3000 + 30
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
        ];

        verify_update_results(&rows, &expected).unwrap();
    }

    async fn test_exec_with_batch_size(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        let updates: Vec<TestData> = initial_data
            .iter()
            .map(|d| TestData {
                id: 0,
                index: d.index,
                key: d.key.clone(),
                name_id: d.name_id * 10,
                name_index: d.name_index / 100,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            })
            .collect();

        let join_on = vec!["index_col".into(), "key_col".into()];
        let set_expr = vec![
            "u.name_id = v.name_id".into(),
            "u.name_index = v.name_index".into(),
        ];

        // Use batch_size=2 to test batching logic (3 rows -> 2+1)
        let result = UpdateBuilder::new(rivetx_sql, "test_data", updates)
            .join_on(join_on)
            .set_expr(set_expr)
            .batch_size(2)
            .exec()
            .await
            .unwrap();

        assert_eq!(result.total_affected, 3);

        let rows = test_data_query_all_no_id(rivetx_sql).await.unwrap();
        let expected = vec![
            TestData {
                id: 0,
                index: 0,
                key: "abc".into(),
                name_id: 10,
                name_index: 10,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 1,
                key: "xyz".into(),
                name_id: 20,
                name_index: 20,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 2,
                key: "def".into(),
                name_id: 30,
                name_index: 30,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
        ];

        verify_update_results(&rows, &expected).unwrap();
    }

    async fn test_exec_with_timeout(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        let updates: Vec<TestData> = initial_data
            .iter()
            .map(|d| TestData {
                id: 0,
                index: d.index,
                key: d.key.clone(),
                name_id: d.name_id * 10,
                name_index: d.name_index / 100,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            })
            .collect();

        let join_on = vec!["index_col".into(), "key_col".into()];
        let set_expr = vec![
            "u.name_id = v.name_id".into(),
            "u.name_index = v.name_index".into(),
        ];

        let result = UpdateBuilder::new(rivetx_sql, "test_data", updates)
            .join_on(join_on)
            .set_expr(set_expr)
            .timeout(Duration::from_secs(30))
            .exec()
            .await
            .unwrap();

        assert_eq!(result.total_affected, 3);

        let rows = test_data_query_all_no_id(rivetx_sql).await.unwrap();
        let expected = vec![
            TestData {
                id: 0,
                index: 0,
                key: "abc".into(),
                name_id: 10,
                name_index: 10,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 1,
                key: "xyz".into(),
                name_id: 20,
                name_index: 20,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 2,
                key: "def".into(),
                name_id: 30,
                name_index: 30,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
        ];

        verify_update_results(&rows, &expected).unwrap();
    }

    async fn test_exec_chained_calls(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        let updates: Vec<TestData> = initial_data
            .iter()
            .map(|d| TestData {
                id: 0,
                index: d.index,
                key: d.key.clone(),
                name_id: d.name_id * 10,
                name_index: d.name_index / 100,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            })
            .collect();

        // Chain all builder methods
        let result = UpdateBuilder::new(rivetx_sql, "test_data", updates)
            .join_on(vec!["index_col".into(), "key_col".into()])
            .set_expr(vec![
                "u.name_id = v.name_id".into(),
                "u.name_index = v.name_index".into(),
            ])
            .batch_size(50)
            .timeout(Duration::from_secs(60))
            .exec()
            .await
            .unwrap();

        assert_eq!(result.total_affected, 3);

        let rows = test_data_query_all_no_id(rivetx_sql).await.unwrap();
        let expected = vec![
            TestData {
                id: 0,
                index: 0,
                key: "abc".into(),
                name_id: 10,
                name_index: 10,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 1,
                key: "xyz".into(),
                name_id: 20,
                name_index: 20,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 2,
                key: "def".into(),
                name_id: 30,
                name_index: 30,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
        ];

        verify_update_results(&rows, &expected).unwrap();
    }

    async fn test_exec_empty_data(rivetx_sql: &crate::conn::RivetxSql) {
        let data: Vec<TestData> = Vec::new();

        let result = UpdateBuilder::new(rivetx_sql, "test_data", data)
            .join_on(vec!["id".into()])
            .set_expr(vec!["u.name_id = v.name_id".into()])
            .exec()
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("vals, join_on, or set_expr is empty"),
            "Expected error about empty vals, got: {}",
            err
        );
    }

    async fn test_exec_empty_join_on(rivetx_sql: &crate::conn::RivetxSql) {
        let data = vec![TestData::default()];

        let result = UpdateBuilder::new(rivetx_sql, "test_data", data)
            .join_on(vec![])
            .set_expr(vec!["u.name_id = v.name_id".into()])
            .exec()
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("vals, join_on, or set_expr is empty"),
            "Expected error about empty join_on, got: {}",
            err
        );
    }

    async fn test_exec_empty_set_expr(rivetx_sql: &crate::conn::RivetxSql) {
        let data = vec![TestData::default()];

        let result = UpdateBuilder::new(rivetx_sql, "test_data", data)
            .join_on(vec!["id".into()])
            .set_expr(vec![])
            .exec()
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("vals, join_on, or set_expr is empty"),
            "Expected error about empty set_expr, got: {}",
            err
        );
    }

    async fn test_exec_zero_batch_size(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        let updates: Vec<TestData> = initial_data
            .iter()
            .map(|d| TestData {
                id: 0,
                index: d.index,
                key: d.key.clone(),
                name_id: d.name_id * 10,
                name_index: d.name_index / 100,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            })
            .collect();

        // batch_size(0) should use default BATCH_SIZE
        let result = UpdateBuilder::new(rivetx_sql, "test_data", updates)
            .batch_size(0)
            .join_on(vec!["index_col".into(), "key_col".into()])
            .set_expr(vec![
                "u.name_id = v.name_id".into(),
                "u.name_index = v.name_index".into(),
            ])
            .exec()
            .await
            .unwrap();

        assert_eq!(result.total_affected, 3);

        let rows = test_data_query_all_no_id(rivetx_sql).await.unwrap();
        let expected = vec![
            TestData {
                id: 0,
                index: 0,
                key: "abc".into(),
                name_id: 10,
                name_index: 10,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 1,
                key: "xyz".into(),
                name_id: 20,
                name_index: 20,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 2,
                key: "def".into(),
                name_id: 30,
                name_index: 30,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
        ];

        verify_update_results(&rows, &expected).unwrap();
    }

    async fn test_exec_zero_timeout(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        let updates: Vec<TestData> = initial_data
            .iter()
            .map(|d| TestData {
                id: 0,
                index: d.index,
                key: d.key.clone(),
                name_id: d.name_id * 10,
                name_index: d.name_index / 100,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            })
            .collect();

        // timeout(0) should use default TIMEOUT
        let result = UpdateBuilder::new(rivetx_sql, "test_data", updates)
            .timeout(Duration::from_secs(0))
            .join_on(vec!["index_col".into(), "key_col".into()])
            .set_expr(vec![
                "u.name_id = v.name_id".into(),
                "u.name_index = v.name_index".into(),
            ])
            .exec()
            .await
            .unwrap();

        assert_eq!(result.total_affected, 3);

        let rows = test_data_query_all_no_id(rivetx_sql).await.unwrap();
        let expected = vec![
            TestData {
                id: 0,
                index: 0,
                key: "abc".into(),
                name_id: 10,
                name_index: 10,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 1,
                key: "xyz".into(),
                name_id: 20,
                name_index: 20,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 2,
                key: "def".into(),
                name_id: 30,
                name_index: 30,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
        ];

        verify_update_results(&rows, &expected).unwrap();
    }

    async fn test_exec_multiple_data_items(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();

        // Insert 5 rows with unique (index_col, key_col) pairs
        let mut test_data = Vec::new();
        for i in 0..5 {
            test_data.push(TestData {
                id: 0,
                index: i,
                key: format!("key_{}", i),
                name_id: i + 1,
                name_index: (i + 1) * 100,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            });
        }

        insert(
            rivetx_sql,
            &"test_data".into(),
            &test_data,
            2,
            &"".into(),
            false,
            Duration::from_secs(10),
        )
        .await
        .unwrap();

        // Update all 5 rows with batch_size=2
        let updates: Vec<TestData> = test_data
            .iter()
            .map(|d| TestData {
                id: 0,
                index: d.index,
                key: d.key.clone(),
                name_id: d.name_id * 100,
                name_index: d.name_index * 2,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            })
            .collect();

        let result = UpdateBuilder::new(rivetx_sql, "test_data", updates)
            .join_on(vec!["index_col".into(), "key_col".into()])
            .set_expr(vec![
                "u.name_id = v.name_id".into(),
                "u.name_index = v.name_index".into(),
            ])
            .batch_size(2)
            .exec()
            .await
            .unwrap();

        assert_eq!(result.total_affected, 5);

        let rows = test_data_query_all_no_id(rivetx_sql).await.unwrap();
        let expected: Vec<TestData> = (0..5)
            .map(|i| TestData {
                id: 0,
                index: i,
                key: format!("key_{}", i),
                name_id: (i + 1) * 100,
                name_index: (i + 1) * 200,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            })
            .collect();

        verify_update_results(&rows, &expected).unwrap();
    }

    // ────────── Default Constants Tests ──────────

    #[test]
    fn test_default_timeout() {
        assert_eq!(TIMEOUT, Duration::from_secs(15));
    }

    #[test]
    fn test_default_batch_size() {
        assert_eq!(BATCH_SIZE, 1024);
    }
}
