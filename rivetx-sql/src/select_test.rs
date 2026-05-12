#[cfg(test)]
mod tests {
    use crate::select::{select_raw, SelectBuilder};
    use crate::sql_value::SqlValue;
    use crate::util::{build_query, QueryCond, BATCH_SIZE, TIMEOUT};
    use crate::util_tests::TestData;
    use mysql_async::Value;
    use rivetx_core::rivetx_string::RivetxString;
    use std::time::Duration;

    // ────────── Helper: build SELECT SQL similar to select_raw ──────────

    /// Build a SELECT SQL string and its arguments, mirroring the logic in select_raw.
    fn build_select_sql(
        table: &str,
        join: &str,
        fields: &[&str],
        g: &QueryCond,
        cond: &str,
        cond_args: &[SqlValue],
        order: &str,
        limit: usize,
        offset: usize,
        batch_size: usize,
    ) -> (String, Vec<SqlValue>) {
        let effective_batch_size = if batch_size == 0 { BATCH_SIZE } else { batch_size };
        let effective_limit = if limit == 0 { usize::MAX } else { limit };
        let min_limit = std::cmp::min(effective_limit, effective_batch_size);
        let limit_clause = format!(" LIMIT {} OFFSET {}", min_limit, offset);

        let fixed_conds: Vec<RivetxString> = g
            .fixed_cols
            .iter()
            .map(|col| format!("{} = ?", col).into())
            .collect();

        let mut tuples: Vec<RivetxString> = Vec::new();
        for vals in &g.in_vals {
            let placeholders = vec!["?"; vals.len()].join(",");
            tuples.push(format!("({})", placeholders).into());
        }

        let mut args: Vec<SqlValue> = Vec::new();
        args.extend(g.fixed_vals.clone());
        args.extend(cond_args.iter().cloned());
        for vals in &g.in_vals {
            args.extend(vals.clone());
        }

        let sql = build_query(
            &["SELECT", &fields.join(", "), "FROM"],
            table,
            join,
            &fixed_conds,
            cond,
            &g.in_cols,
            &tuples,
            order,
            &limit_clause,
        );

        (sql, args)
    }

    // ────────── SQL Building Tests ──────────

    #[test]
    fn test_select_build_with_fixed_cols() {
        let mut g = QueryCond::default();
        g.fixed_cols.push("index_col".into());
        g.fixed_vals.push(SqlValue::from(Value::from(1i32)));

        let fields = ["id", "index_col", "key_col"];
        let (sql, args) = build_select_sql("test_data", "", &fields, &g, "", &[], "", 0, 0, 0);

        assert!(sql.starts_with("SELECT id, index_col, key_col FROM test_data  WHERE"));
        assert!(sql.contains("index_col = ?"));
        assert!(sql.contains("LIMIT"));
        assert!(sql.contains("OFFSET"));
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn test_select_build_with_multiple_fixed_cols() {
        let mut g = QueryCond::default();
        g.fixed_cols.push("index_col".into());
        g.fixed_vals.push(SqlValue::from(Value::from(1i32)));
        g.fixed_cols.push("key_col".into());
        g.fixed_vals.push(SqlValue::from(Value::from("abc")));

        let fields = ["id", "index_col", "key_col"];
        let (sql, args) = build_select_sql("test_data", "", &fields, &g, "", &[], "", 0, 0, 0);

        assert!(sql.contains("index_col = ?"));
        assert!(sql.contains("key_col = ?"));
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_select_build_with_raw_cond() {
        let g = QueryCond::default();
        let fields = ["id", "index_col"];
        let (sql, args) = build_select_sql(
            "test_data",
            "",
            &fields,
            &g,
            "index_col > ?",
            &[SqlValue::from(Value::from(0i32))],
            "",
            0,
            0,
            0,
        );

        assert!(sql.contains("index_col > ?"));
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn test_select_build_with_order() {
        let g = QueryCond::default();
        let fields = ["id", "index_col"];
        let (sql, _args) =
            build_select_sql("test_data", "", &fields, &g, "", &[], "ORDER BY id DESC", 0, 0, 0);

        assert!(sql.contains("ORDER BY id DESC"));
    }

    #[test]
    fn test_select_build_with_limit_offset() {
        let g = QueryCond::default();
        let fields = ["id"];
        let (sql, _args) = build_select_sql("test_data", "", &fields, &g, "", &[], "", 10, 5, 0);

        assert!(sql.contains("LIMIT 10 OFFSET 5"));
    }

    #[test]
    fn test_select_build_with_custom_batch_size() {
        let g = QueryCond::default();
        let fields = ["id"];
        // batch_size=3, limit=10 => min_limit = 3
        let (sql, _args) = build_select_sql("test_data", "", &fields, &g, "", &[], "", 10, 0, 3);

        assert!(sql.contains("LIMIT 3 OFFSET 0"));
    }

    #[test]
    fn test_select_build_with_in_cols_single() {
        let mut g = QueryCond::default();
        g.in_cols.push("id".into());
        g.in_vals
            .push(vec![SqlValue::from(Value::from(1u64))]);
        g.in_vals
            .push(vec![SqlValue::from(Value::from(2u64))]);
        g.in_vals
            .push(vec![SqlValue::from(Value::from(3u64))]);

        let fields = ["id", "index_col"];
        let (sql, args) = build_select_sql("test_data", "", &fields, &g, "", &[], "", 0, 0, 0);

        assert!(sql.contains("(id) IN ((?),(?),(?))"));
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn test_select_build_with_in_cols_multi() {
        let mut g = QueryCond::default();
        g.in_cols.push("key_col".into());
        g.in_cols.push("name_id".into());
        g.in_vals.push(vec![
            SqlValue::from(Value::from("abc")),
            SqlValue::from(Value::from(1u32)),
        ]);
        g.in_vals.push(vec![
            SqlValue::from(Value::from("xyz")),
            SqlValue::from(Value::from(2u32)),
        ]);

        let fields = ["id", "index_col"];
        let (sql, args) = build_select_sql("test_data", "", &fields, &g, "", &[], "", 0, 0, 0);

        assert!(sql.contains("(key_col, name_id) IN ((?,?),(?,?))"));
        assert_eq!(args.len(), 4);
    }

    #[test]
    fn test_select_build_with_fixed_and_in_and_cond() {
        let mut g = QueryCond::default();
        g.fixed_cols.push("status".into());
        g.fixed_vals.push(SqlValue::from(Value::from(1u8)));
        g.in_cols.push("id".into());
        g.in_vals
            .push(vec![SqlValue::from(Value::from(1u64))]);
        g.in_vals
            .push(vec![SqlValue::from(Value::from(2u64))]);

        let fields = ["id"];
        let (sql, args) = build_select_sql(
            "test_data",
            "",
            &fields,
            &g,
            "created_at < NOW()",
            &[],
            "ORDER BY id",
            10,
            0,
            0,
        );

        assert!(sql.contains("status = ?"));
        assert!(sql.contains("created_at < NOW()"));
        assert!(sql.contains("(id) IN"));
        assert!(sql.contains("ORDER BY id"));
        assert!(sql.contains("LIMIT 10 OFFSET 0"));
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn test_select_build_with_join() {
        let g = QueryCond::default();
        let fields = ["d.id", "d.index_col", "d.key_col"];
        let join = "JOIN test_key k ON d.index_col = k.index_col AND d.key_col = k.key_col";
        let (sql, _args) = build_select_sql("test_data d", join, &fields, &g, "", &[], "", 0, 0, 0);

        assert!(sql.contains("FROM test_data d"));
        assert!(sql.contains(
            "JOIN test_key k ON d.index_col = k.index_col AND d.key_col = k.key_col"
        ));
    }

    #[test]
    fn test_select_build_with_all_features() {
        let mut g = QueryCond::default();
        g.fixed_cols.push("d.index_col".into());
        g.fixed_vals.push(SqlValue::from(Value::from(1i32)));
        g.in_cols.push("d.key_col".into());
        g.in_vals
            .push(vec![SqlValue::from(Value::from("abc"))]);
        g.in_vals
            .push(vec![SqlValue::from(Value::from("def"))]);

        let fields = ["d.id", "d.index_col", "d.key_col"];
        let join = "JOIN test_key k ON d.index_col = k.index_col";
        let (sql, args) = build_select_sql(
            "test_data d",
            join,
            &fields,
            &g,
            "d.name_id > ?",
            &[SqlValue::from(Value::from(100i32))],
            "ORDER BY d.id DESC",
            5,
            2,
            10,
        );

        assert!(sql.contains("SELECT d.id, d.index_col, d.key_col FROM"));
        assert!(sql.contains("test_data d"));
        assert!(sql.contains("JOIN test_key k ON d.index_col = k.index_col"));
        assert!(sql.contains("d.index_col = ?"));
        assert!(sql.contains("d.name_id > ?"));
        assert!(sql.contains("(d.key_col) IN ((?),(?))"));
        assert!(sql.contains("ORDER BY d.id DESC"));
        assert!(sql.contains("LIMIT 5 OFFSET 2"));
        // 1 fixed + 1 cond + 2 in = 4 args
        assert_eq!(args.len(), 4);
    }

    // ────────── SelectBuilder Construction Tests ──────────

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
    fn test_select_builder_new() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_table");
        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_where_eq() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data")
            .where_eq("index_col", 1i32)
            .where_eq("key_col", "abc");

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_where_in_single_col() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data").where_in(
            vec!["id".into()],
            vec![
                vec![SqlValue::from(Value::from(1u64))],
                vec![SqlValue::from(Value::from(2u64))],
                vec![SqlValue::from(Value::from(3u64))],
            ],
        );

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_where_in_multi_col() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data").where_in(
            vec!["key_col".into(), "name_id".into()],
            vec![
                vec![
                    SqlValue::from(Value::from("abc")),
                    SqlValue::from(Value::from(1u32)),
                ],
                vec![
                    SqlValue::from(Value::from("xyz")),
                    SqlValue::from(Value::from(2u32)),
                ],
            ],
        );

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_where_in_batch_size() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data")
            .where_in_batch_size(512);

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_where_cond() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data")
            .where_cond("index_col = ?", vec![SqlValue::from(Value::from(1i32))]);

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_where_cond_multiple() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data")
            .where_cond("index_col = ?", vec![SqlValue::from(Value::from(1i32))])
            .where_cond(
                "and key_col = ?",
                vec![SqlValue::from(Value::from("abc"))],
            );

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_order() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data")
            .order("order by index_col, key_col");

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_limit() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data").limit(100);

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_offset() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data").offset(50);

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_timeout() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data")
            .timeout(Duration::from_secs(30));

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_batch_size() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data").batch_size(512);

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_order_field_select() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data")
            .order_field_select("id", true, 10);

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_order_field_select_asc() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data")
            .order_field_select("id", false, 20);

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_chained_calls() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data")
            .where_eq("index_col", 1i32)
            .where_in(
                vec!["key_col".into()],
                vec![
                    vec![SqlValue::from(Value::from("abc"))],
                    vec![SqlValue::from(Value::from("def"))],
                ],
            )
            .where_cond(
                "name_id > ?",
                vec![SqlValue::from(Value::from(100i32))],
            )
            .order("order by index_col, key_col")
            .limit(10)
            .offset(5)
            .batch_size(128)
            .timeout(Duration::from_secs(60));

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_where_eq_different_types() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data")
            .where_eq("id", 1u64)
            .where_eq("name_id", 42i32)
            .where_eq("key_col", "hello")
            .where_eq("is_active", true);

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_where_in_empty_vals() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data")
            .where_in(vec!["id".into()], vec![]);

        let _builder: SelectBuilder<TestData> = builder;
    }

    #[test]
    fn test_select_builder_join() {
        let rivetx_sql = make_mock_sql();
        let builder = SelectBuilder::<TestData>::new(&rivetx_sql, "test_data d").join(
            "JOIN test_key k ON d.index_col = k.index_col AND d.key_col = k.key_col",
        );

        let _builder: SelectBuilder<TestData> = builder;
    }

    // ────────── select_raw Validation Tests ──────────

    /// Test that select_raw rejects (order/limit/offset) with in_batch_size > 0.
    /// This test verifies the validation logic without needing a DB connection,
    /// by checking that the error is returned before any DB operation.
    #[tokio::test]
    async fn test_select_raw_rejects_order_with_in_batch() {
        let rivetx_sql = make_mock_sql();
        let mut cond = QueryCond::new();
        cond.fixed_cols.push("id".into());
        cond.fixed_vals.push(SqlValue::from(Value::from(1u64)));
        cond.in_batch_size = 100;

        let result = select_raw::<TestData>(
            &rivetx_sql,
            &"test_data".into(),
            &"".into(),
            &cond,
            &"".into(),
            &vec![],
            &"order by id".into(),
            10,
            0,
            0,
            Duration::from_secs(10),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("order/limit/offset"),
            "Expected error about order/limit/offset, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_select_raw_rejects_limit_with_in_batch() {
        let rivetx_sql = make_mock_sql();
        let mut cond = QueryCond::new();
        cond.fixed_cols.push("id".into());
        cond.fixed_vals.push(SqlValue::from(Value::from(1u64)));
        cond.in_batch_size = 100;

        let result = select_raw::<TestData>(
            &rivetx_sql,
            &"test_data".into(),
            &"".into(),
            &cond,
            &"".into(),
            &vec![],
            &"".into(),
            10,
            0,
            0,
            Duration::from_secs(10),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("order/limit/offset"),
            "Expected error about order/limit/offset, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_select_raw_rejects_offset_with_in_batch() {
        let rivetx_sql = make_mock_sql();
        let mut cond = QueryCond::new();
        cond.fixed_cols.push("id".into());
        cond.fixed_vals.push(SqlValue::from(Value::from(1u64)));
        cond.in_batch_size = 100;

        let result = select_raw::<TestData>(
            &rivetx_sql,
            &"test_data".into(),
            &"".into(),
            &cond,
            &"".into(),
            &vec![],
            &"".into(),
            0,
            5,
            0,
            Duration::from_secs(10),
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("order/limit/offset"),
            "Expected error about order/limit/offset, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_select_raw_rejects_fixed_cols_vals_mismatch() {
        let rivetx_sql = make_mock_sql();
        let mut cond = QueryCond::new();
        cond.fixed_cols.push("id".into());
        cond.fixed_cols.push("key_col".into());
        // Only one value for two cols -> mismatch
        cond.fixed_vals.push(SqlValue::from(Value::from(1u64)));

        let result = select_raw::<TestData>(
            &rivetx_sql,
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
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("fixedCols and fixedVals length mismatch"),
            "Expected mismatch error, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_select_raw_rejects_in_cols_without_in_vals() {
        let rivetx_sql = make_mock_sql();
        let mut cond = QueryCond::new();
        cond.fixed_cols.push("id".into());
        cond.fixed_vals.push(SqlValue::from(Value::from(1u64)));
        cond.in_cols.push("key_col".into());
        // in_vals is empty -> error

        let result = select_raw::<TestData>(
            &rivetx_sql,
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
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("len(queryCond.InCols) > 0 && len(queryCond.InVals) == 0"),
            "Expected error about empty InVals, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_select_raw_rejects_in_cols_vals_length_mismatch() {
        let rivetx_sql = make_mock_sql();
        let mut cond = QueryCond::new();
        cond.fixed_cols.push("id".into());
        cond.fixed_vals.push(SqlValue::from(Value::from(1u64)));
        cond.in_cols.push("key_col".into());
        cond.in_cols.push("name_id".into());
        // Each in_vals row should have 2 elements, but we provide only 1
        cond.in_vals
            .push(vec![SqlValue::from(Value::from("abc"))]);

        let result = select_raw::<TestData>(
            &rivetx_sql,
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
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("len(queryCond.InCols) != len(queryCond.InVals[0])"),
            "Expected error about InCols/InVals length mismatch, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_select_raw_rejects_in_vals_row_length_mismatch() {
        let rivetx_sql = make_mock_sql();
        let mut cond = QueryCond::new();
        cond.fixed_cols.push("id".into());
        cond.fixed_vals.push(SqlValue::from(Value::from(1u64)));
        cond.in_cols.push("key_col".into());
        cond.in_cols.push("name_id".into());
        // First row has 2 elements (correct), second row has 1 element (wrong)
        cond.in_vals.push(vec![
            SqlValue::from(Value::from("abc")),
            SqlValue::from(Value::from(1u32)),
        ]);
        cond.in_vals.push(vec![SqlValue::from(Value::from("xyz"))]);

        let result = select_raw::<TestData>(
            &rivetx_sql,
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
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("does not match InCols length"),
            "Expected error about InVals row length mismatch, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_select_raw_rejects_empty_conditions() {
        let rivetx_sql = make_mock_sql();
        let cond = QueryCond::new();

        let result = select_raw::<TestData>(
            &rivetx_sql,
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
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("both FixedCols and InCols and cond are empty"),
            "Expected error about empty conditions, got: {}",
            err
        );
    }

    // ────────── QueryCond Tests ──────────

    #[test]
    fn test_query_cond_new() {
        let cond = QueryCond::new();
        assert!(cond.fixed_cols.is_empty());
        assert!(cond.fixed_vals.is_empty());
        assert!(cond.in_cols.is_empty());
        assert!(cond.in_vals.is_empty());
        assert_eq!(cond.in_batch_size, 0);
    }

    #[test]
    fn test_query_cond_default() {
        let cond = QueryCond::default();
        assert!(cond.fixed_cols.is_empty());
        assert!(cond.fixed_vals.is_empty());
        assert!(cond.in_cols.is_empty());
        assert!(cond.in_vals.is_empty());
        assert_eq!(cond.in_batch_size, 0);
    }

    #[test]
    fn test_query_cond_with_values() {
        let mut cond = QueryCond::new();
        cond.fixed_cols.push("id".into());
        cond.fixed_vals.push(SqlValue::from(Value::from(1u64)));
        cond.in_cols.push("key_col".into());
        cond.in_vals
            .push(vec![SqlValue::from(Value::from("abc"))]);
        cond.in_batch_size = 100;

        assert_eq!(cond.fixed_cols.len(), 1);
        assert_eq!(cond.fixed_vals.len(), 1);
        assert_eq!(cond.in_cols.len(), 1);
        assert_eq!(cond.in_vals.len(), 1);
        assert_eq!(cond.in_batch_size, 100);
    }

    // ────────── build_query Edge Cases ──────────

    #[test]
    fn test_build_query_with_empty_join() {
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "1=1",
            &[],
            &[],
            "",
            "",
        );
        // Note: build_query adds a space after table name, then " WHERE" (with leading space)
        assert_eq!(sql, "SELECT * FROM test_data  WHERE 1=1");
    }

    #[test]
    fn test_build_query_with_only_limit() {
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "1=1",
            &[],
            &[],
            "",
            " LIMIT 10",
        );
        assert_eq!(sql, "SELECT * FROM test_data  WHERE 1=1  LIMIT 10");
    }

    #[test]
    fn test_build_query_with_order_and_limit() {
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "1=1",
            &[],
            &[],
            "ORDER BY id DESC",
            " LIMIT 5",
        );
        assert_eq!(
            sql,
            "SELECT * FROM test_data  WHERE 1=1 ORDER BY id DESC  LIMIT 5"
        );
    }

    #[test]
    fn test_build_query_with_fixed_cond_only() {
        let fixed = vec![RivetxString::from("id = ?")];
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &fixed,
            "",
            &[],
            &[],
            "",
            "",
        );
        assert_eq!(sql, "SELECT * FROM test_data  WHERE id = ?");
    }

    #[test]
    fn test_build_query_with_multiple_fixed_conds() {
        let fixed = vec![
            RivetxString::from("index_col = ?"),
            RivetxString::from("key_col = ?"),
        ];
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &fixed,
            "",
            &[],
            &[],
            "",
            "",
        );
        assert_eq!(
            sql,
            "SELECT * FROM test_data  WHERE index_col = ? AND key_col = ?"
        );
    }

    #[test]
    fn test_build_query_with_in_tuples() {
        let in_cols = vec![RivetxString::from("id")];
        let tuples = vec![
            RivetxString::from("(?)"),
            RivetxString::from("(?)"),
            RivetxString::from("(?)"),
        ];
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "1=1",
            &in_cols,
            &tuples,
            "",
            "",
        );
        assert_eq!(
            sql,
            "SELECT * FROM test_data  WHERE 1=1 AND (id) IN ((?),(?),(?))"
        );
    }

    #[test]
    fn test_build_query_with_multi_col_in_tuples() {
        let in_cols = vec![
            RivetxString::from("key_col"),
            RivetxString::from("name_id"),
        ];
        let tuples = vec![
            RivetxString::from("(?,?)"),
            RivetxString::from("(?,?)"),
        ];
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "1=1",
            &in_cols,
            &tuples,
            "",
            "",
        );
        assert_eq!(
            sql,
            "SELECT * FROM test_data  WHERE 1=1 AND (key_col, name_id) IN ((?,?),(?,?))"
        );
    }

    // ────────── TIMEOUT / BATCH_SIZE Constants ──────────

    #[test]
    fn test_default_timeout() {
        assert_eq!(TIMEOUT, Duration::from_secs(15));
    }

    #[test]
    fn test_default_batch_size() {
        assert_eq!(BATCH_SIZE, 1024);
    }
}
