#[cfg(test)]
mod tests {
    use crate::select::{select_raw, SelectBuilder};
    use crate::sql_value::SqlValue;
    use crate::util::{build_query, QueryCond, BATCH_SIZE, TIMEOUT};
    use crate::util_tests::{test_open_rivetx_sql_sync, TestData};
    use anyhow::Context;
    use chrono::Timelike;
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
        let effective_batch_size = if batch_size == 0 {
            BATCH_SIZE
        } else {
            batch_size
        };
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
        let (sql, _args) = build_select_sql(
            "test_data",
            "",
            &fields,
            &g,
            "",
            &[],
            "ORDER BY id DESC",
            0,
            0,
            0,
        );

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
        g.in_vals.push(vec![SqlValue::from(Value::from(1u64))]);
        g.in_vals.push(vec![SqlValue::from(Value::from(2u64))]);
        g.in_vals.push(vec![SqlValue::from(Value::from(3u64))]);

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
        g.in_vals.push(vec![SqlValue::from(Value::from(1u64))]);
        g.in_vals.push(vec![SqlValue::from(Value::from(2u64))]);

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
        assert!(
            sql.contains("JOIN test_key k ON d.index_col = k.index_col AND d.key_col = k.key_col")
        );
    }

    #[test]
    fn test_select_build_with_all_features() {
        let mut g = QueryCond::default();
        g.fixed_cols.push("d.index_col".into());
        g.fixed_vals.push(SqlValue::from(Value::from(1i32)));
        g.in_cols.push("d.key_col".into());
        g.in_vals.push(vec![SqlValue::from(Value::from("abc"))]);
        g.in_vals.push(vec![SqlValue::from(Value::from("def"))]);

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

    // ────────── SelectBuilder Integration Tests (with real DB) ──────────

    /// Helper to create a mock RivetxSql for builder construction.
    fn make_mock_sql() -> crate::conn::RivetxSql {
        test_open_rivetx_sql_sync().expect("Failed to create mock RivetxSql")
    }

    /// Helper: insert initial test data for select tests
    async fn insert_initial_data(
        rivetx_sql: &crate::conn::RivetxSql,
        curr_time: chrono::NaiveDateTime,
    ) -> anyhow::Result<Vec<TestData>> {
        let test_data = vec![
            TestData {
                id: 0,
                index: 1,
                key: "abc".into(),
                name_id: 100,
                name_index: 1000,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 1,
                key: "def".into(),
                name_id: 101,
                name_index: 1001,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 2,
                key: "ghi".into(),
                name_id: 102,
                name_index: 1002,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 2,
                key: "xyz".into(),
                name_id: 103,
                name_index: 1003,
                curr_time: curr_time.clone(),
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
        ];

        crate::insert::insert(
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

        Ok(test_data)
    }

    /// Helper: create table and clear data for a test
    async fn setup_table(rivetx_sql: &crate::conn::RivetxSql) {
        crate::util_tests::test_data_create_table(rivetx_sql)
            .await
            .unwrap();
        crate::util_tests::test_data_clear_table(rivetx_sql)
            .await
            .unwrap();
    }

    /// Helper: get current time truncated to seconds
    fn now_truncated() -> chrono::NaiveDateTime {
        chrono::Local::now()
            .naive_local()
            .with_nanosecond(0)
            .unwrap()
    }

    fn zero_naive_date_time() -> chrono::NaiveDateTime {
        crate::util_tests::zero_naive_date_time()
    }

    /// Single entry point for all SelectBuilder integration tests.
    /// Runs sequentially to avoid DB race conditions.
    #[tokio::test]
    async fn test_select_builder_integration() {
        let _guard = crate::util_tests::lock_test_db();
        let rivetx_sql = make_mock_sql();

        // Run all integration tests sequentially
        test_exec_basic(&rivetx_sql).await;
        test_exec_where_eq(&rivetx_sql).await;
        test_exec_where_cond(&rivetx_sql).await;
        test_exec_where_cond_multiple(&rivetx_sql).await;
        test_exec_order(&rivetx_sql).await;
        test_exec_limit_offset(&rivetx_sql).await;
        test_exec_chained_calls(&rivetx_sql).await;
        test_exec_where_in_single_col(&rivetx_sql).await;
        test_exec_where_in_multi_col(&rivetx_sql).await;
        test_exec_where_in_rows_same_type_array(&rivetx_sql).await;
        test_exec_where_in_empty_vals(&rivetx_sql).await;
        test_exec_where_eq_different_types(&rivetx_sql).await;
        test_exec_with_batch_size(&rivetx_sql).await;
        test_exec_with_timeout(&rivetx_sql).await;
        test_exec_order_field_select_desc(&rivetx_sql).await;
        test_exec_order_field_select_asc(&rivetx_sql).await;
    }

    async fn test_exec_basic(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        // Query all rows with a condition that matches everything
        let result: Vec<TestData> = SelectBuilder::new("test_data")
            .where_eq("1", 1)
            .order("order by index_col, key_col")
            .exec(rivetx_sql)
            .await
            .unwrap();

        assert_eq!(result.len(), initial_data.len());
        for (i, row) in result.iter().enumerate() {
            assert_eq!(row.index, initial_data[i].index);
            assert_eq!(row.key, initial_data[i].key);
            assert_eq!(row.name_id, initial_data[i].name_id);
        }
    }

    async fn test_exec_where_eq(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let _initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        // Query with where_eq on index_col = 1 (should return 2 rows: abc, def)
        let result: Vec<TestData> = SelectBuilder::new("test_data")
            .where_eq("index_col", 1i32)
            .order("order by key_col")
            .exec(rivetx_sql)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, "abc");
        assert_eq!(result[1].key, "def");
    }

    async fn test_exec_where_cond(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let _initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        // Query with where_cond
        let result: Vec<TestData> = SelectBuilder::new("test_data")
            .where_cond("index_col = ?", vec![1i32.into()])
            .where_cond("and key_col = ?", vec!["abc".into()])
            .exec(rivetx_sql)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, "abc");
        assert_eq!(result[0].name_id, 100);
    }

    async fn test_exec_where_cond_multiple(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let _initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        // Query with combined where_cond in a single call
        let result: Vec<TestData> = SelectBuilder::new("test_data")
            .where_cond(
                "index_col = ? and key_col = ?",
                vec![1i32.into(), "abc".into()],
            )
            .exec(rivetx_sql)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, "abc");
    }

    async fn test_exec_order(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let _initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        // Query with order by key_col DESC
        let result: Vec<TestData> = SelectBuilder::new("test_data")
            .where_eq("index_col", 1i32)
            .order("order by key_col desc")
            .exec(rivetx_sql)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, "def");
        assert_eq!(result[1].key, "abc");
    }

    async fn test_exec_limit_offset(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let _initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        // Query with limit and offset
        let result: Vec<TestData> = SelectBuilder::new("test_data")
            .where_eq("1", 1)
            .order("order by index_col, key_col")
            .limit(2)
            .offset(1)
            .exec(rivetx_sql)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        // offset 1 means skip "abc", so first result should be "def"
        assert_eq!(result[0].key, "def");
        assert_eq!(result[1].key, "ghi");
    }

    async fn test_exec_chained_calls(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let _initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        // Chain all builder methods
        let result: Vec<TestData> = SelectBuilder::new("test_data")
            .where_eq("index_col", 1i32)
            .where_in("key_col", ["abc", "def"])
            .where_cond("name_id > ?", vec![SqlValue::from(Value::from(50i32))])
            .order("order by key_col")
            .limit(10)
            .offset(0)
            .batch_size(128)
            .timeout(Duration::from_secs(60))
            .exec(rivetx_sql)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, "abc");
        assert_eq!(result[1].key, "def");
    }

    async fn test_exec_where_in_single_col(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let _initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        // Query with where_in on single column
        let result: Vec<TestData> = SelectBuilder::new("test_data")
            .where_eq("index_col", 1i32)
            .where_in("key_col", ["abc", "def"])
            .order("order by key_col")
            .exec(rivetx_sql)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, "abc");
        assert_eq!(result[1].key, "def");
    }

    async fn test_exec_where_in_multi_col(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let _initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        // Query with where_in_rows on mixed-type columns (&str, i32)
        let result: Vec<TestData> = SelectBuilder::new("test_data")
            .where_in_rows(
                ["key_col", "name_id"],
                vec![("abc", 100i32), ("def", 101i32)],
            )
            .order("order by key_col")
            .exec(rivetx_sql)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, "abc");
        assert_eq!(result[1].key, "def");
    }

    async fn test_exec_where_in_rows_same_type_array(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let _initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        let result: Vec<TestData> = SelectBuilder::new("test_data")
            .where_in_rows(["name_id", "name_index"], vec![[100i32, 1000], [101, 1001]])
            .order("order by key_col")
            .exec(rivetx_sql)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, "abc");
        assert_eq!(result[1].key, "def");
    }

    async fn test_exec_where_in_empty_vals(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let _initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        // Query with empty where_in should return an error
        let result = SelectBuilder::<TestData>::new("test_data")
            .where_eq("index_col", 1i32)
            .where_in("key_col", Vec::<&str>::new())
            .exec(rivetx_sql)
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("len(queryCond.InCols) > 0 && len(queryCond.InVals) == 0"),
            "Expected error about empty InVals, got: {}",
            err
        );
    }

    async fn test_exec_where_eq_different_types(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let _initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        // Query with where_eq on different types
        let result: Vec<TestData> = SelectBuilder::new("test_data")
            .where_eq("index_col", 1i32)
            .where_eq("key_col", "abc")
            .exec(rivetx_sql)
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].key, "abc");
        assert_eq!(result[0].name_id, 100);
    }

    async fn test_exec_with_batch_size(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let _initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        // Query with custom batch_size
        let result: Vec<TestData> = SelectBuilder::new("test_data")
            .where_eq("1", 1)
            .order("order by index_col, key_col")
            .batch_size(2)
            .exec(rivetx_sql)
            .await
            .unwrap();

        assert_eq!(result.len(), 4);
    }

    async fn test_exec_with_timeout(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let _initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        // Query with custom timeout
        let result: Vec<TestData> = SelectBuilder::new("test_data")
            .where_eq("1", 1)
            .order("order by index_col, key_col")
            .timeout(Duration::from_secs(30))
            .exec(rivetx_sql)
            .await
            .unwrap();

        assert_eq!(result.len(), 4);
    }

    async fn test_exec_order_field_select_desc(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let _initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        // Query with order_field_select DESC (limit 2)
        let result: Vec<TestData> = SelectBuilder::new("test_data")
            .order_field_select("id", true, 2)
            .timeout(Duration::from_secs(10))
            .exec(rivetx_sql)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        // DESC order: highest ids first
        assert!(result[0].id > result[1].id);
    }

    async fn test_exec_order_field_select_asc(rivetx_sql: &crate::conn::RivetxSql) {
        setup_table(rivetx_sql).await;

        let curr_time = now_truncated();
        let _initial_data = insert_initial_data(rivetx_sql, curr_time.clone())
            .await
            .unwrap();

        // Query with order_field_select ASC (limit 2)
        let result: Vec<TestData> = SelectBuilder::new("test_data")
            .order_field_select("id", false, 2)
            .timeout(Duration::from_secs(10))
            .exec(rivetx_sql)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        // ASC order: lowest ids first
        assert!(result[0].id < result[1].id);
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
        cond.in_vals.push(vec![SqlValue::from(Value::from("abc"))]);

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
        cond.in_vals.push(vec![SqlValue::from(Value::from("abc"))]);
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
        let in_cols = vec![RivetxString::from("key_col"), RivetxString::from("name_id")];
        let tuples = vec![RivetxString::from("(?,?)"), RivetxString::from("(?,?)")];
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
        assert_eq!(TIMEOUT, Duration::from_secs(120));
    }

    #[test]
    fn test_default_batch_size() {
        assert_eq!(BATCH_SIZE, 1024);
    }
}
