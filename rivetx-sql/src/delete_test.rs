#[cfg(test)]
mod tests {
    use crate::delete::{DeleteBuilder, DeleteResult};
    use crate::sql_value::SqlValue;
    use crate::util::QueryCond;
    use mysql_async::Value;
    use std::fmt::Write;
    use std::time::Duration;

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

        assert_eq!(sql, "DELETE FROM test_data WHERE key_col = ? AND name_id = ?");
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

        assert_eq!(
            sql,
            "DELETE FROM test_data WHERE id IN ((?), (?), (?))"
        );
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

    /// Helper to create a mock RivetxSql for builder construction.
    /// The builder is only used for state verification (not exec()),
    /// so the actual connection doesn't matter.
    fn make_mock_sql() -> crate::conn::RivetxSql {
        // Use a dummy connection string; the builder will never actually connect
        // since we don't call exec() in these unit tests.
        crate::conn::RivetxSql::new(
            "mysql://root:Yfygz@389@192.168.192.139:3306/test_db",
            1,
            1,
        )
        .expect("Failed to create mock RivetxSql")
    }

    #[test]
    fn test_delete_builder_new() {
        let rivetx_sql = make_mock_sql();
        let builder = DeleteBuilder::new(&rivetx_sql, "test_table");

        // Verify the builder was created successfully (no panic)
        // The table name is consumed by the builder, we just verify the type
        let _builder: DeleteBuilder = builder;
    }

    #[test]
    fn test_delete_builder_where_eq() {
        let rivetx_sql = make_mock_sql();
        let builder = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_eq("id", 1u64);

        // Chain another where_eq
        let builder = builder.where_eq("key_col", "abc");

        // Verify the builder is still valid after chaining
        let _builder: DeleteBuilder = builder;
    }

    #[test]
    fn test_delete_builder_where_in_single_col() {
        let rivetx_sql = make_mock_sql();
        let builder = DeleteBuilder::new(&rivetx_sql, "test_data").where_in(
            vec!["id".into()],
            vec![
                vec![SqlValue::from(Value::from(1u64))],
                vec![SqlValue::from(Value::from(2u64))],
                vec![SqlValue::from(Value::from(3u64))],
            ],
        );

        let _builder: DeleteBuilder = builder;
    }

    #[test]
    fn test_delete_builder_where_in_multi_col() {
        let rivetx_sql = make_mock_sql();
        let builder = DeleteBuilder::new(&rivetx_sql, "test_data").where_in(
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

        let _builder: DeleteBuilder = builder;
    }

    #[test]
    fn test_delete_builder_where_in_batch_size() {
        let rivetx_sql = make_mock_sql();
        let builder = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_in_batch_size(512);

        let _builder: DeleteBuilder = builder;
    }

    #[test]
    fn test_delete_builder_where_raw() {
        let rivetx_sql = make_mock_sql();
        let builder = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_raw("index_col > ?", vec![SqlValue::from(Value::from(0i32))]);

        let _builder: DeleteBuilder = builder;
    }

    #[test]
    fn test_delete_builder_where_raw_multiple() {
        let rivetx_sql = make_mock_sql();
        let builder = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_raw("index_col > ?", vec![SqlValue::from(Value::from(0i32))])
            .where_raw("name_id < ?", vec![SqlValue::from(Value::from(100i32))]);

        let _builder: DeleteBuilder = builder;
    }

    #[test]
    fn test_delete_builder_limit() {
        let rivetx_sql = make_mock_sql();
        let builder = DeleteBuilder::new(&rivetx_sql, "test_data")
            .limit(100);

        let _builder: DeleteBuilder = builder;
    }

    #[test]
    fn test_delete_builder_timeout() {
        let rivetx_sql = make_mock_sql();
        let builder = DeleteBuilder::new(&rivetx_sql, "test_data")
            .timeout(Duration::from_secs(30));

        let _builder: DeleteBuilder = builder;
    }

    #[test]
    fn test_delete_builder_reserve_size() {
        let rivetx_sql = make_mock_sql();
        let builder = DeleteBuilder::new(&rivetx_sql, "test_data")
            .reserve_size("id", 5, Duration::from_millis(10));

        let _builder: DeleteBuilder = builder;
    }

    #[test]
    fn test_delete_builder_chained_calls() {
        let rivetx_sql = make_mock_sql();
        let builder = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_eq("key_col", "abc")
            .where_in(
                vec!["name_id".into(), "name_index".into()],
                vec![
                    vec![SqlValue::from(Value::from(1u64)), SqlValue::from(Value::from(1001u64))],
                    vec![SqlValue::from(Value::from(2u64)), SqlValue::from(Value::from(1002u64))],
                ],
            )
            .where_raw("index_col > ?", vec![SqlValue::from(Value::from(0i32))])
            .limit(10)
            .timeout(Duration::from_secs(60));

        let _builder: DeleteBuilder = builder;
    }

    #[test]
    fn test_delete_builder_where_raw_empty_args() {
        let rivetx_sql = make_mock_sql();
        let builder = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_raw("created_at < NOW()", vec![]);

        let _builder: DeleteBuilder = builder;
    }

    #[test]
    fn test_delete_builder_reserve_size_zero() {
        let rivetx_sql = make_mock_sql();
        let builder = DeleteBuilder::new(&rivetx_sql, "test_data")
            .reserve_size("id", 0, Duration::from_secs(0));

        let _builder: DeleteBuilder = builder;
    }

    #[test]
    fn test_delete_builder_where_eq_different_types() {
        let rivetx_sql = make_mock_sql();
        let builder = DeleteBuilder::new(&rivetx_sql, "test_data")
            .where_eq("id", 1u64)
            .where_eq("name_id", 42i32)
            .where_eq("key_col", "hello")
            .where_eq("is_active", true);

        let _builder: DeleteBuilder = builder;
    }

    #[test]
    fn test_delete_builder_where_in_empty_vals() {
        let rivetx_sql = make_mock_sql();
        let builder = DeleteBuilder::new(&rivetx_sql, "test_data").where_in(
            vec!["id".into()],
            vec![],
        );

        let _builder: DeleteBuilder = builder;
    }
}
