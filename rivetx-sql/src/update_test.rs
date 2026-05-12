#[cfg(test)]
mod tests {
    use crate::update::{update_raw, UpdateBuilder, UpdateResult};
    use crate::util::{BATCH_SIZE, TIMEOUT};
    use crate::util_tests::TestData;
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

    // ────────── UpdateBuilder Construction Tests ──────────

    /// Helper to create a mock RivetxSql for builder construction.
    /// The builder is only used for state verification (not exec()),
    /// so the actual connection doesn't matter.
    fn make_mock_sql() -> crate::conn::RivetxSql {
        crate::conn::RivetxSql::new(
            "mysql://root:Yfygz@389@192.168.192.139:3306/test_db",
            1,
            1,
        )
        .expect("Failed to create mock RivetxSql")
    }

    #[test]
    fn test_update_builder_new() {
        let rivetx_sql = make_mock_sql();
        let data = vec![TestData::default()];
        let builder = UpdateBuilder::new(&rivetx_sql, "test_table", data);

        let _builder: UpdateBuilder<TestData> = builder;
    }

    #[test]
    fn test_update_builder_join_on() {
        let rivetx_sql = make_mock_sql();
        let data = vec![TestData::default()];
        let builder = UpdateBuilder::new(&rivetx_sql, "test_data", data)
            .join_on(vec!["index_col".into(), "key_col".into()]);

        let _builder: UpdateBuilder<TestData> = builder;
    }

    #[test]
    fn test_update_builder_set_expr() {
        let rivetx_sql = make_mock_sql();
        let data = vec![TestData::default()];
        let builder = UpdateBuilder::new(&rivetx_sql, "test_data", data)
            .set_expr(vec![
                "u.name_id = v.name_id".into(),
                "u.name_index = u.name_index + v.name_index".into(),
            ]);

        let _builder: UpdateBuilder<TestData> = builder;
    }

    #[test]
    fn test_update_builder_batch_size() {
        let rivetx_sql = make_mock_sql();
        let data = vec![TestData::default()];
        let builder = UpdateBuilder::new(&rivetx_sql, "test_data", data)
            .batch_size(100);

        let _builder: UpdateBuilder<TestData> = builder;
    }

    #[test]
    fn test_update_builder_timeout() {
        let rivetx_sql = make_mock_sql();
        let data = vec![TestData::default()];
        let builder = UpdateBuilder::new(&rivetx_sql, "test_data", data)
            .timeout(Duration::from_secs(30));

        let _builder: UpdateBuilder<TestData> = builder;
    }

    #[test]
    fn test_update_builder_chained_calls() {
        let rivetx_sql = make_mock_sql();
        let data = vec![TestData::default()];
        let builder = UpdateBuilder::new(&rivetx_sql, "test_data", data)
            .join_on(vec!["index_col".into(), "key_col".into()])
            .set_expr(vec![
                "u.name_id = v.name_id".into(),
                "u.name_index = u.name_index + v.name_index".into(),
            ])
            .batch_size(50)
            .timeout(Duration::from_secs(60));

        let _builder: UpdateBuilder<TestData> = builder;
    }

    #[test]
    fn test_update_builder_empty_data() {
        let rivetx_sql = make_mock_sql();
        let data: Vec<TestData> = Vec::new();
        let builder = UpdateBuilder::new(&rivetx_sql, "test_data", data)
            .join_on(vec!["id".into()])
            .set_expr(vec!["u.name_id = v.name_id".into()]);

        let _builder: UpdateBuilder<TestData> = builder;
    }

    #[test]
    fn test_update_builder_multiple_data_items() {
        let rivetx_sql = make_mock_sql();
        let data = vec![TestData::default(), TestData::default(), TestData::default()];
        let builder = UpdateBuilder::new(&rivetx_sql, "test_data", data)
            .join_on(vec!["index_col".into(), "key_col".into()])
            .set_expr(vec!["u.name_id = v.name_id".into()])
            .batch_size(2);

        let _builder: UpdateBuilder<TestData> = builder;
    }

    #[test]
    fn test_update_builder_join_on_empty() {
        let rivetx_sql = make_mock_sql();
        let data = vec![TestData::default()];
        let builder = UpdateBuilder::new(&rivetx_sql, "test_data", data)
            .join_on(vec![])
            .set_expr(vec!["u.name_id = v.name_id".into()]);

        let _builder: UpdateBuilder<TestData> = builder;
    }

    #[test]
    fn test_update_builder_set_expr_empty() {
        let rivetx_sql = make_mock_sql();
        let data = vec![TestData::default()];
        let builder = UpdateBuilder::new(&rivetx_sql, "test_data", data)
            .join_on(vec!["id".into()])
            .set_expr(vec![]);

        let _builder: UpdateBuilder<TestData> = builder;
    }

    #[test]
    fn test_update_builder_zero_batch_size() {
        let rivetx_sql = make_mock_sql();
        let data = vec![TestData::default()];
        let builder = UpdateBuilder::new(&rivetx_sql, "test_data", data)
            .batch_size(0)
            .join_on(vec!["id".into()])
            .set_expr(vec!["u.name_id = v.name_id".into()]);

        let _builder: UpdateBuilder<TestData> = builder;
    }

    #[test]
    fn test_update_builder_zero_timeout() {
        let rivetx_sql = make_mock_sql();
        let data = vec![TestData::default()];
        let builder = UpdateBuilder::new(&rivetx_sql, "test_data", data)
            .timeout(Duration::from_secs(0))
            .join_on(vec!["id".into()])
            .set_expr(vec!["u.name_id = v.name_id".into()]);

        let _builder: UpdateBuilder<TestData> = builder;
    }

    // ────────── update_raw Validation Tests ──────────

    /// Test that update_raw rejects empty vals.
    /// This test verifies the validation logic without needing a DB connection,
    /// by checking that the error is returned before any DB operation.
    #[tokio::test]
    async fn test_update_raw_rejects_empty_vals() {
        let rivetx_sql = make_mock_sql();
        let cols: Vec<RivetxString> = vec!["id".into(), "name_id".into()];
        let vals: Vec<Vec<Value>> = Vec::new();
        let join_on: Vec<RivetxString> = vec!["id".into()];
        let set_expr: Vec<RivetxString> = vec!["u.name_id = v.name_id".into()];

        let result = update_raw(
            &rivetx_sql,
            &"test_data".into(),
            &cols,
            vals,
            &join_on,
            &set_expr,
            0,
            Duration::from_secs(0),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("vals, cols, join_on, or set_expr is empty"),
            "Expected error about empty vals, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_update_raw_rejects_empty_cols() {
        let rivetx_sql = make_mock_sql();
        let cols: Vec<RivetxString> = Vec::new();
        let vals = vec![vec![Value::from(1u64), Value::from(100i32)]];
        let join_on: Vec<RivetxString> = vec!["id".into()];
        let set_expr: Vec<RivetxString> = vec!["u.name_id = v.name_id".into()];

        let result = update_raw(
            &rivetx_sql,
            &"test_data".into(),
            &cols,
            vals,
            &join_on,
            &set_expr,
            0,
            Duration::from_secs(0),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("vals, cols, join_on, or set_expr is empty"),
            "Expected error about empty cols, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_update_raw_rejects_empty_join_on() {
        let rivetx_sql = make_mock_sql();
        let cols: Vec<RivetxString> = vec!["id".into(), "name_id".into()];
        let vals = vec![vec![Value::from(1u64), Value::from(100i32)]];
        let join_on: Vec<RivetxString> = Vec::new();
        let set_expr: Vec<RivetxString> = vec!["u.name_id = v.name_id".into()];

        let result = update_raw(
            &rivetx_sql,
            &"test_data".into(),
            &cols,
            vals,
            &join_on,
            &set_expr,
            0,
            Duration::from_secs(0),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("vals, cols, join_on, or set_expr is empty"),
            "Expected error about empty join_on, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_update_raw_rejects_empty_set_expr() {
        let rivetx_sql = make_mock_sql();
        let cols: Vec<RivetxString> = vec!["id".into(), "name_id".into()];
        let vals = vec![vec![Value::from(1u64), Value::from(100i32)]];
        let join_on: Vec<RivetxString> = vec!["id".into()];
        let set_expr: Vec<RivetxString> = Vec::new();

        let result = update_raw(
            &rivetx_sql,
            &"test_data".into(),
            &cols,
            vals,
            &join_on,
            &set_expr,
            0,
            Duration::from_secs(0),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("vals, cols, join_on, or set_expr is empty"),
            "Expected error about empty set_expr, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_update_raw_rejects_vals_cols_length_mismatch() {
        let rivetx_sql = make_mock_sql();
        let cols: Vec<RivetxString> = vec!["id".into(), "name_id".into(), "name_index".into()];
        // Row has only 2 values, but cols has 3
        let vals = vec![vec![Value::from(1u64), Value::from(100i32)]];
        let join_on: Vec<RivetxString> = vec!["id".into()];
        let set_expr: Vec<RivetxString> = vec!["u.name_id = v.name_id".into()];

        let result = update_raw(
            &rivetx_sql,
            &"test_data".into(),
            &cols,
            vals,
            &join_on,
            &set_expr,
            0,
            Duration::from_secs(0),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("does not match cols length"),
            "Expected error about length mismatch, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_update_raw_rejects_multiple_vals_cols_mismatch() {
        let rivetx_sql = make_mock_sql();
        let cols: Vec<RivetxString> = vec!["id".into(), "name_id".into()];
        let vals = vec![
            vec![Value::from(1u64), Value::from(100i32)], // 2 values, correct
            vec![Value::from(2u64)],                       // 1 value, wrong
        ];
        let join_on: Vec<RivetxString> = vec!["id".into()];
        let set_expr: Vec<RivetxString> = vec!["u.name_id = v.name_id".into()];

        let result = update_raw(
            &rivetx_sql,
            &"test_data".into(),
            &cols,
            vals,
            &join_on,
            &set_expr,
            0,
            Duration::from_secs(0),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("does not match cols length"),
            "Expected error about length mismatch, got: {}",
            err
        );
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
