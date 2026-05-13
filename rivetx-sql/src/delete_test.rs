#[cfg(test)]
mod tests {
    use crate::delete::{DeleteBuilder, DeleteResult};
    use crate::sql_value::SqlValue;
    use crate::util::QueryCond;
    use crate::util_tests::{
        test_data_clear_table, test_data_count_rows, test_data_create_table, test_open_rivetx_sql,
        zero_naive_date_time, TestData,
    };
    use crate::{insert::insert, util_tests::test_data_query_all_no_id};
    use chrono::Timelike;
    use mysql_async::Value;
    use std::fmt::Write;
    use std::time::Duration;

    // These exec() tests share the same physical table (`test_data`) and therefore must not run
    // concurrently, otherwise they can interleave truncate/insert and violate unique constraints.
    static TEST_DATA_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn lock_test_data() -> std::sync::MutexGuard<'static, ()> {
        TEST_DATA_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Helper function to build DELETE SQL similar to delete_raw
    fn build_delete_sql(
        table: &str,
        g: &QueryCond,
        cond: &str,
        limit: usize,
    ) -> Result<(String, Vec<SqlValue>), std::fmt::Error> {
        let mut sql = format!("DELETE FROM {}", table);
        let mut where_parts: Vec<String> = Vec::with_capacity(128);
        let mut args: Vec<SqlValue> = Vec::new();

        // Fixed Cols
        for (col, val) in g.fixed_cols.iter().zip(g.fixed_vals.iter()) {
            where_parts.push(format!("{} = ?", col));
            args.push(val.clone());
        }

        // Cond
        if !cond.is_empty() {
            where_parts.push(cond.to_string());
        }

        // InCols
        if !g.in_cols.is_empty() && !g.in_vals.is_empty() {
            let col_part = if g.in_cols.len() > 1 {
                format!("({})", g.in_cols.join(", "))
            } else {
                g.in_cols[0].to_string()
            };

            let mut tuples = Vec::with_capacity(128);
            for row_vals in &g.in_vals {
                let placeholders = vec!["?"; row_vals.len()].join(", ");
                tuples.push(format!("({})", placeholders));
                args.extend(row_vals.clone());
            }
            where_parts.push(format!("{} IN ({})", col_part, tuples.join(", ")));
        }

        if !where_parts.is_empty() {
            write!(sql, " WHERE {}", where_parts.join(" AND "))?;
        }

        if limit > 0 {
            write!(sql, " LIMIT {}", limit)?;
        }

        Ok((sql, args))
    }

    // ────────── SQL Building Tests ──────────

    #[test]
    fn test_delete_with_fixed_cols() {
        let mut g = QueryCond::default();
        g.fixed_cols.push("id".into());
        g.fixed_vals.push(SqlValue::from(Value::from(1u64)));

        let (sql, args) = build_delete_sql("test_data", &g, "", 0).unwrap();

        assert_eq!(sql, "DELETE FROM test_data WHERE id = ?");
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn test_delete_with_multiple_fixed_cols() {
        let mut g = QueryCond::default();
        g.fixed_cols.push("key_col".into());
        g.fixed_cols.push("name_id".into());
        g.fixed_vals.push(SqlValue::from(Value::from("abc")));
        g.fixed_vals.push(SqlValue::from(Value::from(1u32)));

        let (sql, args) = build_delete_sql("test_data", &g, "", 0).unwrap();

        assert_eq!(
            sql,
            "DELETE FROM test_data WHERE key_col = ? AND name_id = ?"
        );
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_delete_with_in_cols_single_col() {
        let mut g = QueryCond::default();
        g.in_cols.push("id".into());
        g.in_vals.push(vec![SqlValue::from(Value::from(1u64))]);
        g.in_vals.push(vec![SqlValue::from(Value::from(2u64))]);
        g.in_vals.push(vec![SqlValue::from(Value::from(3u64))]);

        let (sql, args) = build_delete_sql("test_data", &g, "", 0).unwrap();

        assert_eq!(sql, "DELETE FROM test_data WHERE id IN ((?), (?), (?))");
        assert_eq!(args.len(), 3);
    }

    #[test]
    fn test_delete_with_in_cols_multiple_cols() {
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

        let (sql, args) = build_delete_sql("test_data", &g, "", 0).unwrap();

        assert_eq!(
            sql,
            "DELETE FROM test_data WHERE (key_col, name_id) IN ((?, ?), (?, ?))"
        );
        assert_eq!(args.len(), 4);
    }

    #[test]
    fn test_delete_with_raw_cond() {
        let g = QueryCond::default();
        let (sql, args) = build_delete_sql("test_data", &g, "index_col > ?", 0).unwrap();

        assert_eq!(sql, "DELETE FROM test_data WHERE index_col > ?");
        assert_eq!(args.len(), 0);
    }

    #[test]
    fn test_delete_with_fixed_and_raw_cond() {
        let mut g = QueryCond::default();
        g.fixed_cols.push("key_col".into());
        g.fixed_vals.push(SqlValue::from(Value::from("abc")));

        let (sql, args) = build_delete_sql("test_data", &g, "name_id > ?", 0).unwrap();

        assert_eq!(
            sql,
            "DELETE FROM test_data WHERE key_col = ? AND name_id > ?"
        );
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn test_delete_with_limit() {
        let mut g = QueryCond::default();
        g.fixed_cols.push("id".into());
        g.fixed_vals.push(SqlValue::from(Value::from(1u64)));

        let (sql, args) = build_delete_sql("test_data", &g, "", 100).unwrap();

        assert_eq!(sql, "DELETE FROM test_data WHERE id = ? LIMIT 100");
        assert_eq!(args.len(), 1);
    }

    #[test]
    fn test_delete_with_all_conditions() {
        let mut g = QueryCond::default();

        // Fixed condition
        g.fixed_cols.push("status".into());
        g.fixed_vals.push(SqlValue::from(Value::from(0u8)));

        // IN condition
        g.in_cols.push("id".into());
        g.in_vals.push(vec![SqlValue::from(Value::from(1u64))]);
        g.in_vals.push(vec![SqlValue::from(Value::from(2u64))]);

        let (sql, args) = build_delete_sql("test_data", &g, "created_at < NOW()", 50).unwrap();

        assert!(sql.contains("DELETE FROM test_data WHERE"));
        assert!(sql.contains("status = ?"));
        assert!(sql.contains("created_at < NOW()"));
        assert!(sql.contains("id IN"));
        assert!(sql.contains("LIMIT 50"));
        assert_eq!(args.len(), 3); // 1 fixed + 2 in values
    }

    #[test]
    fn test_delete_with_empty_conditions() {
        let g = QueryCond::default();
        let (sql, args) = build_delete_sql("test_data", &g, "", 0).unwrap();

        assert_eq!(sql, "DELETE FROM test_data");
        assert_eq!(args.len(), 0);
    }

    #[test]
    fn test_delete_batch_size_property() {
        let mut g = QueryCond::default();
        g.in_batch_size = 512;

        assert_eq!(g.in_batch_size, 512);
    }

    // ────────── DeleteResult Tests ──────────

    #[test]
    fn test_delete_result_default() {
        let result = DeleteResult::default();

        assert_eq!(result.total_affected, 0);
        assert_eq!(result.last_insert_id, 0);
    }

    #[test]
    fn test_delete_result_creation() {
        let result = DeleteResult {
            total_affected: 42,
            last_insert_id: 100,
        };

        assert_eq!(result.total_affected, 42);
        assert_eq!(result.last_insert_id, 100);
    }

    // ────────── DeleteBuilder Tests ──────────

    #[tokio::test]
    async fn test_delete_builder_new_exec_err_on_empty_conditions() -> anyhow::Result<()> {
        let _guard = lock_test_data();
        let rivetx_sql = test_open_rivetx_sql().await?;
        // This should error because fixed/in/cond are all empty.
        assert!(DeleteBuilder::new(&rivetx_sql, "test_data")
            .exec()
            .await
            .is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_where_eq_exec() -> anyhow::Result<()> {
        let _guard = lock_test_data();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_eq("index_col", 0i32)
            .where_eq("key_col", "abc")
            .exec()
            .await?;
        assert_eq!(res.total_affected, 1);

        let count = test_data_count_rows(&rivetx_sql, "test_data").await;
        assert_eq!(count, 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_where_in_single_col_exec() -> anyhow::Result<()> {
        let _guard = lock_test_data();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_in(vec!["name_id".into()], vec![vec![1.into()], vec![3.into()]])
            .exec()
            .await?;
        assert_eq!(res.total_affected, 2);

        let count = test_data_count_rows(&rivetx_sql, "test_data").await;
        assert_eq!(count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_where_in_multi_col_exec() -> anyhow::Result<()> {
        let _guard = lock_test_data();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_in(
                vec!["name_id".into(), "name_index".into()],
                vec![vec![1.into(), 1001.into()], vec![2.into(), 1002.into()]],
            )
            .exec()
            .await?;
        assert_eq!(res.total_affected, 2);

        let count = test_data_count_rows(&rivetx_sql, "test_data").await;
        assert_eq!(count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_where_in_batch_size_exec() -> anyhow::Result<()> {
        let _guard = lock_test_data();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_in_batch_size(1)
            .where_in(vec!["name_id".into()], vec![vec![1.into()], vec![2.into()]])
            .exec()
            .await?;
        assert_eq!(res.total_affected, 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_where_raw_exec() -> anyhow::Result<()> {
        let _guard = lock_test_data();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_raw("index_col > ?", vec![SqlValue::from(Value::from(0i32))])
            .exec()
            .await?;
        assert_eq!(res.total_affected, 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_where_raw_multiple_exec() -> anyhow::Result<()> {
        let _guard = lock_test_data();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_raw("index_col > ?", vec![SqlValue::from(Value::from(0i32))])
            .where_raw("name_id < ?", vec![SqlValue::from(Value::from(3i32))])
            .exec()
            .await?;
        assert_eq!(res.total_affected, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_limit_exec() -> anyhow::Result<()> {
        let _guard = lock_test_data();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_eq("key_col", "abc")
            .limit(100)
            .exec()
            .await?;
        assert_eq!(res.total_affected, 2);

        let count = test_data_count_rows(&rivetx_sql, "test_data").await;
        assert_eq!(count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_timeout_exec() -> anyhow::Result<()> {
        let _guard = lock_test_data();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_eq("key_col", "abc")
            .timeout(Duration::from_secs(5))
            .exec()
            .await?;
        assert_eq!(res.total_affected, 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_reserve_size_exec() -> anyhow::Result<()> {
        let _guard = lock_test_data();
        let rivetx_sql = test_open_rivetx_sql().await?;

        test_data_create_table(&rivetx_sql).await?;
        test_data_clear_table(&rivetx_sql).await?;

        let now = chrono::Local::now()
            .naive_local()
            .with_nanosecond(0)
            .unwrap();
        let mut rows = Vec::new();
        for i in 0..6 {
            rows.push(TestData {
                id: 0,
                index: i,
                key: "abc".into(),
                name_id: 100 + i,
                name_index: 2000 + i,
                curr_time: now,
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            });
        }

        insert(
            &rivetx_sql,
            &"test_data".into(),
            &rows,
            2,
            &"".into(),
            false,
            Duration::from_secs(10),
        )
        .await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .reserve_size("id", 2, Duration::from_millis(1))
            .exec()
            .await?;
        assert_eq!(res.total_affected, 4);

        let count = test_data_count_rows(&rivetx_sql, "test_data").await;
        assert_eq!(count, 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_chained_calls_exec() -> anyhow::Result<()> {
        let _guard = lock_test_data();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_eq("key_col", "abc")
            .where_in(
                vec!["name_id".into(), "name_index".into()],
                vec![vec![1.into(), 1001.into()], vec![2.into(), 1002.into()]],
            )
            .where_raw("index_col >= ?", vec![SqlValue::from(Value::from(0i32))])
            .limit(10)
            .timeout(Duration::from_secs(5))
            .exec()
            .await?;

        assert_eq!(res.total_affected, 2);
        let count = test_data_count_rows(&rivetx_sql, "test_data").await;
        assert_eq!(count, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_where_raw_empty_args_exec() -> anyhow::Result<()> {
        let _guard = lock_test_data();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_raw("1=1", vec![])
            .exec()
            .await?;
        assert_eq!(res.total_affected, 3);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_reserve_size_zero_exec() -> anyhow::Result<()> {
        let _guard = lock_test_data();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .reserve_size("id", 0, Duration::from_millis(1))
            .exec()
            .await?;
        assert_eq!(res.total_affected, 3);

        let count = test_data_count_rows(&rivetx_sql, "test_data").await;
        assert_eq!(count, 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_where_eq_different_types_exec() -> anyhow::Result<()> {
        let _guard = lock_test_data();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_eq("index_col", 1i32)
            .where_eq("name_id", 2i32)
            .where_eq("key_col", "abc")
            .exec()
            .await?;
        assert_eq!(res.total_affected, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_where_in_empty_vals_exec() -> anyhow::Result<()> {
        let _guard = lock_test_data();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        // Current behavior: in_cols is set but in_vals is empty, which produces no WHERE clause.
        // That becomes a full-table delete. This test locks down the behavior explicitly.
        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_in(vec!["name_id".into()], vec![])
            .exec()
            .await?;
        assert_eq!(res.total_affected, 3);

        let count = test_data_count_rows(&rivetx_sql, "test_data").await;
        assert_eq!(count, 0);
        Ok(())
    }

    // ────────── DeleteBuilder exec() Integration Tests ──────────

    async fn seed_test_data(rivetx_sql: &crate::conn::RivetxSql) -> anyhow::Result<()> {
        test_data_create_table(rivetx_sql).await?;
        test_data_clear_table(rivetx_sql).await?;

        let now = chrono::Local::now()
            .naive_local()
            .with_nanosecond(0)
            .unwrap();
        let rows = vec![
            TestData {
                id: 0,
                index: 0,
                key: "abc".into(),
                name_id: 1,
                name_index: 1001,
                curr_time: now,
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 1,
                key: "abc".into(),
                name_id: 2,
                name_index: 1002,
                curr_time: now,
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
            TestData {
                id: 0,
                index: 2,
                key: "xyz".into(),
                name_id: 3,
                name_index: 1003,
                curr_time: now,
                created_at: zero_naive_date_time(),
                updated_at: zero_naive_date_time(),
            },
        ];

        insert(
            rivetx_sql,
            &"test_data".into(),
            &rows,
            2,
            &"".into(),
            false,
            Duration::from_secs(10),
        )
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_exec_where_eq_affects_one() -> anyhow::Result<()> {
        let _guard = TEST_DATA_LOCK.lock().unwrap();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let before = test_data_count_rows(&rivetx_sql, "test_data").await;
        assert_eq!(before, 3);

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_eq("index_col", 1)
            .where_eq("key_col", "abc")
            .exec()
            .await?;
        assert_eq!(res.total_affected, 1);

        let after = test_data_count_rows(&rivetx_sql, "test_data").await;
        assert_eq!(after, 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_exec_where_in_affects_two() -> anyhow::Result<()> {
        let _guard = TEST_DATA_LOCK.lock().unwrap();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_in(
                vec!["name_id".into(), "name_index".into()],
                vec![vec![1.into(), 1001.into()], vec![2.into(), 1002.into()]],
            )
            .exec()
            .await?;
        assert_eq!(res.total_affected, 2);

        let after = test_data_count_rows(&rivetx_sql, "test_data").await;
        assert_eq!(after, 1);

        // Remaining row should be the "xyz" one
        let rows = test_data_query_all_no_id(&rivetx_sql).await?;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].index, 2);
        assert_eq!(rows[0].key, "xyz");
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_exec_where_raw_with_args() -> anyhow::Result<()> {
        let _guard = TEST_DATA_LOCK.lock().unwrap();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_raw("name_id >= ?", vec![SqlValue::from(Value::from(2i32))])
            .exec()
            .await?;

        // name_id: 2,3 should be removed => 2 rows
        assert_eq!(res.total_affected, 2);
        let after = test_data_count_rows(&rivetx_sql, "test_data").await;
        assert_eq!(after, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_builder_exec_limit() -> anyhow::Result<()> {
        let _guard = TEST_DATA_LOCK.lock().unwrap();
        let rivetx_sql = test_open_rivetx_sql().await?;
        seed_test_data(&rivetx_sql).await?;

        let res = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_eq("key_col", "abc")
            .limit(1)
            .exec()
            .await?;

        assert_eq!(res.total_affected, 1);
        let after = test_data_count_rows(&rivetx_sql, "test_data").await;
        assert_eq!(after, 2);
        Ok(())
    }
}
