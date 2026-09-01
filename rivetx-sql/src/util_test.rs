#[cfg(test)]
mod tests {
    use crate::util::{build_query, to_snake_case, QueryCond, QueryStruct};
    use rivetx_core::rivetx_string::RivetxString;

    // ────────── QueryCond Tests ──────────

    #[test]
    fn test_query_cond_new_creates_empty_instance() {
        let cond = QueryCond::new();
        assert!(cond.fixed_cols.is_empty());
        assert!(cond.fixed_vals.is_empty());
        assert!(cond.in_cols.is_empty());
        assert!(cond.in_vals.is_empty());
        assert_eq!(cond.in_batch_size, 0);
    }

    #[test]
    fn test_query_cond_default_creates_empty_instance() {
        let cond = QueryCond::default();
        assert!(cond.fixed_cols.is_empty());
        assert!(cond.fixed_vals.is_empty());
        assert!(cond.in_cols.is_empty());
        assert!(cond.in_vals.is_empty());
        assert_eq!(cond.in_batch_size, 0);
    }

    #[test]
    fn test_query_cond_new_and_default_are_equivalent() {
        let cond_new = QueryCond::new();
        let cond_default = QueryCond::default();
        assert_eq!(cond_new.fixed_cols.len(), cond_default.fixed_cols.len());
        assert_eq!(cond_new.fixed_vals.len(), cond_default.fixed_vals.len());
        assert_eq!(cond_new.in_cols.len(), cond_default.in_cols.len());
        assert_eq!(cond_new.in_vals.len(), cond_default.in_vals.len());
        assert_eq!(cond_new.in_batch_size, cond_default.in_batch_size);
    }

    #[test]
    fn test_query_cond_with_fixed_conditions() {
        let mut cond = QueryCond::new();
        cond.fixed_cols.push("index_col".into());
        cond.fixed_cols.push("key_col".into());
        cond.fixed_vals.push(1i32.into());
        cond.fixed_vals.push("abc".into());

        assert_eq!(cond.fixed_cols.len(), 2);
        assert_eq!(cond.fixed_vals.len(), 2);
        assert_eq!(cond.fixed_cols[0], "index_col");
        assert_eq!(cond.fixed_cols[1], "key_col");
    }

    #[test]
    fn test_query_cond_with_in_conditions() {
        let mut cond = QueryCond::new();
        cond.in_cols.push("id".into());
        cond.in_vals
            .push(vec![1u64.into(), 2u64.into(), 3u64.into()]);

        assert_eq!(cond.in_cols.len(), 1);
        assert_eq!(cond.in_vals.len(), 1);
        assert_eq!(cond.in_vals[0].len(), 3);
    }

    #[test]
    fn test_query_cond_with_in_batch_size() {
        let mut cond = QueryCond::new();
        cond.in_batch_size = 100;
        assert_eq!(cond.in_batch_size, 100);
    }

    #[test]
    fn test_query_cond_clone_is_independent() {
        let mut cond = QueryCond::new();
        cond.fixed_cols.push("id".into());
        cond.fixed_vals.push(1u64.into());

        let mut cloned = cond.clone();
        cloned.fixed_cols.push("key_col".into());
        cloned.fixed_vals.push("abc".into());

        // Original should still have only 1 element
        assert_eq!(cond.fixed_cols.len(), 1);
        assert_eq!(cond.fixed_vals.len(), 1);
        // Clone should have 2 elements
        assert_eq!(cloned.fixed_cols.len(), 2);
        assert_eq!(cloned.fixed_vals.len(), 2);
    }

    #[test]
    fn test_query_cond_set_in_single_column() {
        let mut cond = QueryCond::new();
        cond.set_in("aaa", ["1", "2", "3"]);
        assert_eq!(cond.in_cols, vec![RivetxString::from("aaa")]);
        assert_eq!(cond.in_vals.len(), 3);
        assert_eq!(cond.in_vals[0].len(), 1);
        assert_eq!(cond.in_vals[1].len(), 1);
        assert_eq!(cond.in_vals[2].len(), 1);
    }

    #[test]
    fn test_query_cond_set_in_rows_mixed_types() {
        let mut cond = QueryCond::new();
        cond.set_in_rows(["id", "name"], vec![(1i64, "a"), (2i64, "b")]);
        assert_eq!(cond.in_cols.len(), 2);
        assert_eq!(cond.in_vals.len(), 2);
        assert_eq!(cond.in_vals[0].len(), 2);
        assert_eq!(cond.in_vals[1].len(), 2);
    }

    // ────────── QueryStruct Tests ──────────

    #[test]
    fn test_query_struct_default() {
        let qs: QueryStruct<i32, String> = QueryStruct::default();
        assert!(qs.fixed.is_none());
        assert!(qs.in_vals.is_empty());
    }

    #[test]
    fn test_query_struct_with_fixed_value() {
        let mut qs: QueryStruct<i32, String> = QueryStruct::default();
        qs.fixed = Some(42);
        assert_eq!(qs.fixed, Some(42));
        assert!(qs.in_vals.is_empty());
    }

    #[test]
    fn test_query_struct_with_in_vals() {
        let mut qs: QueryStruct<i32, String> = QueryStruct::default();
        qs.in_vals.push("abc".to_string());
        qs.in_vals.push("def".to_string());
        assert_eq!(qs.in_vals.len(), 2);
        assert_eq!(qs.in_vals[0], "abc");
        assert_eq!(qs.in_vals[1], "def");
    }

    #[test]
    fn test_query_struct_with_both_fixed_and_in_vals() {
        let mut qs: QueryStruct<String, i32> = QueryStruct::default();
        qs.fixed = Some("status".to_string());
        qs.in_vals.push(1);
        qs.in_vals.push(2);
        qs.in_vals.push(3);

        assert_eq!(qs.fixed, Some("status".to_string()));
        assert_eq!(qs.in_vals.len(), 3);
    }

    // ────────── to_snake_case Tests ──────────

    #[test]
    fn test_to_snake_case_single_word_lowercase() {
        assert_eq!(to_snake_case("hello"), "hello");
    }

    #[test]
    fn test_to_snake_case_single_word_uppercase() {
        assert_eq!(to_snake_case("HELLO"), "h_e_l_l_o");
    }

    #[test]
    fn test_to_snake_case_camel_case() {
        assert_eq!(to_snake_case("helloWorld"), "hello_world");
    }

    #[test]
    fn test_to_snake_case_multiple_uppercase() {
        assert_eq!(to_snake_case("helloWorldAgain"), "hello_world_again");
    }

    #[test]
    fn test_to_snake_case_all_uppercase_word() {
        assert_eq!(to_snake_case("XMLParser"), "x_m_l_parser");
    }

    #[test]
    fn test_to_snake_case_single_character() {
        assert_eq!(to_snake_case("A"), "a");
    }

    #[test]
    fn test_to_snake_case_empty_string() {
        assert_eq!(to_snake_case(""), "");
    }

    #[test]
    fn test_to_snake_case_already_snake_case() {
        assert_eq!(to_snake_case("hello_world"), "hello_world");
    }

    #[test]
    fn test_to_snake_case_with_numbers() {
        assert_eq!(to_snake_case("hello2World"), "hello2_world");
    }

    #[test]
    fn test_to_snake_case_trailing_uppercase() {
        assert_eq!(to_snake_case("helloWORLD"), "hello_w_o_r_l_d");
    }

    #[test]
    fn test_to_snake_case_leading_uppercase() {
        assert_eq!(to_snake_case("HelloWorld"), "hello_world");
    }

    #[test]
    fn test_to_snake_case_consecutive_uppercase() {
        assert_eq!(to_snake_case("parseXMLDocument"), "parse_x_m_l_document");
    }

    #[test]
    fn test_to_snake_case_unicode() {
        assert_eq!(to_snake_case("caféConLeche"), "café_con_leche");
    }

    // ────────── build_query Tests ──────────

    // ── Basic Structure ──

    #[test]
    fn test_build_query_basic_select() {
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "",
            &[],
            &[],
            "",
            "",
        );
        assert!(sql.starts_with("SELECT * FROM test_data"));
        assert!(sql.contains("WHERE"));
    }

    #[test]
    fn test_build_query_select_with_specific_fields() {
        let sql = build_query(
            &["SELECT", "id, name", "FROM"],
            "users",
            "",
            &[],
            "",
            &[],
            &[],
            "",
            "",
        );
        assert!(sql.starts_with("SELECT id, name FROM users"));
        assert!(sql.contains("WHERE"));
    }

    // ── Fixed Conditions ──

    #[test]
    fn test_build_query_with_single_fixed_cond() {
        let fixed_conds = vec![RivetxString::from("id = ?")];
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &fixed_conds,
            "",
            &[],
            &[],
            "",
            "",
        );
        assert!(sql.contains("WHERE id = ?"));
    }

    #[test]
    fn test_build_query_with_multiple_fixed_conds() {
        let fixed_conds = vec![
            RivetxString::from("index_col = ?"),
            RivetxString::from("key_col = ?"),
        ];
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &fixed_conds,
            "",
            &[],
            &[],
            "",
            "",
        );
        assert!(sql.contains("WHERE index_col = ? AND key_col = ?"));
    }

    #[test]
    fn test_build_query_with_three_fixed_conds() {
        let fixed_conds = vec![
            RivetxString::from("a = ?"),
            RivetxString::from("b = ?"),
            RivetxString::from("c = ?"),
        ];
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "t",
            "",
            &fixed_conds,
            "",
            &[],
            &[],
            "",
            "",
        );
        assert!(sql.contains("WHERE a = ? AND b = ? AND c = ?"));
    }

    // ── Raw Condition ──

    #[test]
    fn test_build_query_with_raw_cond() {
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "index_col > ?",
            &[],
            &[],
            "",
            "",
        );
        assert!(sql.contains("WHERE index_col > ?"));
    }

    #[test]
    fn test_build_query_with_raw_cond_and_fixed_conds() {
        let fixed_conds = vec![RivetxString::from("status = ?")];
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &fixed_conds,
            "created_at < NOW()",
            &[],
            &[],
            "",
            "",
        );
        assert!(sql.contains("WHERE status = ? AND created_at < NOW()"));
    }

    // ── IN Conditions ──

    #[test]
    fn test_build_query_with_single_in_col() {
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
            "",
            &in_cols,
            &tuples,
            "",
            "",
        );
        assert!(sql.contains("(id) IN ((?),(?),(?))"));
    }

    #[test]
    fn test_build_query_with_multi_col_in() {
        let in_cols = vec![RivetxString::from("key_col"), RivetxString::from("name_id")];
        let tuples = vec![RivetxString::from("(?,?)"), RivetxString::from("(?,?)")];
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "",
            &in_cols,
            &tuples,
            "",
            "",
        );
        assert!(sql.contains("(key_col, name_id) IN ((?,?),(?,?))"));
    }

    #[test]
    fn test_build_query_with_single_in_tuple() {
        let in_cols = vec![RivetxString::from("id")];
        let tuples = vec![RivetxString::from("(?)")];
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "",
            &in_cols,
            &tuples,
            "",
            "",
        );
        assert!(sql.contains("(id) IN ((?))"));
    }

    // ── Order and Limit ──

    #[test]
    fn test_build_query_with_order() {
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "",
            &[],
            &[],
            "ORDER BY id DESC",
            "",
        );
        assert!(sql.contains("ORDER BY id DESC"));
    }

    #[test]
    fn test_build_query_with_limit() {
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "",
            &[],
            &[],
            "",
            "LIMIT 10",
        );
        assert!(sql.contains("LIMIT 10"));
    }

    #[test]
    fn test_build_query_with_order_and_limit() {
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "",
            &[],
            &[],
            "ORDER BY id",
            "LIMIT 10 OFFSET 5",
        );
        assert!(sql.contains("ORDER BY id"));
        assert!(sql.contains("LIMIT 10 OFFSET 5"));
    }

    // ── Join ──

    #[test]
    fn test_build_query_with_join() {
        let sql = build_query(
            &["SELECT", "d.*", "FROM"],
            "test_data d",
            "JOIN test_key k ON d.index_col = k.index_col",
            &[],
            "",
            &[],
            &[],
            "",
            "",
        );
        assert!(sql.contains("FROM test_data d"));
        assert!(sql.contains("JOIN test_key k ON d.index_col = k.index_col"));
    }

    // ── Combined Features ──

    #[test]
    fn test_build_query_with_all_features() {
        let fixed_conds = vec![RivetxString::from("d.status = ?")];
        let in_cols = vec![RivetxString::from("d.key_col")];
        let tuples = vec![RivetxString::from("(?)"), RivetxString::from("(?)")];

        let sql = build_query(
            &["SELECT", "d.id, d.name", "FROM"],
            "test_data d",
            "JOIN test_key k ON d.id = k.id",
            &fixed_conds,
            "d.age > ?",
            &in_cols,
            &tuples,
            "ORDER BY d.id",
            "LIMIT 5 OFFSET 0",
        );

        assert!(sql.starts_with("SELECT d.id, d.name FROM"));
        assert!(sql.contains("test_data d"));
        assert!(sql.contains("JOIN test_key k ON d.id = k.id"));
        assert!(sql.contains("WHERE d.status = ? AND d.age > ? AND (d.key_col) IN ((?),(?))"));
        assert!(sql.contains("ORDER BY d.id"));
        assert!(sql.contains("LIMIT 5 OFFSET 0"));
    }

    // ── Edge Cases ──

    #[test]
    fn test_build_query_empty_sqls() {
        let sql = build_query(&[], "test_data", "", &[], "", &[], &[], "", "");
        // Should still produce "test_data WHERE"
        assert!(sql.contains("test_data"));
        assert!(sql.contains("WHERE"));
    }

    #[test]
    fn test_build_query_no_conditions() {
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "",
            &[],
            &[],
            "",
            "",
        );
        assert_eq!(sql, "SELECT * FROM test_data  WHERE");
    }

    #[test]
    fn test_build_query_with_empty_fixed_conds_and_empty_cond() {
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "",
            &[],
            &[],
            "",
            "",
        );
        // No conditions added, just "WHERE" with nothing after
        assert_eq!(sql, "SELECT * FROM test_data  WHERE");
    }

    #[test]
    fn test_build_query_with_only_order() {
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "",
            &[],
            &[],
            "ORDER BY id",
            "",
        );
        assert!(sql.contains("ORDER BY id"));
        assert!(sql.contains("WHERE"));
    }

    #[test]
    fn test_build_query_with_only_limit() {
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "",
            &[],
            &[],
            "",
            "LIMIT 100",
        );
        assert!(sql.contains("LIMIT 100"));
        assert!(sql.contains("WHERE"));
    }

    #[test]
    fn test_build_query_with_join_and_all_conditions() {
        let fixed_conds = vec![RivetxString::from("d.index_col = ?")];
        let in_cols = vec![RivetxString::from("d.key_col")];
        let tuples = vec![RivetxString::from("(?)")];

        let sql = build_query(
            &["SELECT", "d.*", "FROM"],
            "test_data d",
            "LEFT JOIN other o ON d.id = o.id",
            &fixed_conds,
            "d.name LIKE ?",
            &in_cols,
            &tuples,
            "ORDER BY d.id ASC",
            "LIMIT 1",
        );

        assert!(sql.contains("LEFT JOIN other o ON d.id = o.id"));
        assert!(sql.contains("d.index_col = ?"));
        assert!(sql.contains("d.name LIKE ?"));
        assert!(sql.contains("(d.key_col) IN ((?))"));
        assert!(sql.contains("ORDER BY d.id ASC"));
        assert!(sql.contains("LIMIT 1"));
    }

    #[test]
    fn test_build_query_where_clause_position() {
        // Verify WHERE appears before any conditions
        let fixed_conds = vec![RivetxString::from("x = ?")];
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "t",
            "",
            &fixed_conds,
            "",
            &[],
            &[],
            "",
            "",
        );
        // WHERE should come before the condition
        let where_pos = sql.find("WHERE").unwrap();
        let cond_pos = sql.find("x = ?").unwrap();
        assert!(
            where_pos < cond_pos,
            "WHERE should appear before conditions"
        );
    }

    #[test]
    fn test_build_query_in_condition_uses_and() {
        let in_cols = vec![RivetxString::from("id")];
        let tuples = vec![RivetxString::from("(?)")];
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "t",
            "",
            &[],
            "",
            &in_cols,
            &tuples,
            "",
            "",
        );
        // IN clause should be preceded by "AND" when there are no fixed conds or raw cond
        // Actually, when there are no fixed_conds and no raw cond, the IN clause is the first condition
        // so it should just have a space before "("
        assert!(sql.contains("WHERE (id) IN"));
    }

    #[test]
    fn test_build_query_in_condition_with_fixed_uses_and() {
        let fixed_conds = vec![RivetxString::from("status = ?")];
        let in_cols = vec![RivetxString::from("id")];
        let tuples = vec![RivetxString::from("(?)")];
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "t",
            "",
            &fixed_conds,
            "",
            &in_cols,
            &tuples,
            "",
            "",
        );
        // IN clause should be preceded by "AND" when there are fixed conds
        assert!(sql.contains("status = ? AND (id) IN"));
    }

    #[test]
    fn test_build_query_output_does_not_have_trailing_whitespace() {
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "test_data",
            "",
            &[],
            "",
            &[],
            &[],
            "",
            "",
        );
        assert!(
            !sql.ends_with(' '),
            "SQL should not have trailing whitespace"
        );
    }

    #[test]
    fn test_build_query_with_table_name_containing_underscores() {
        let sql = build_query(
            &["SELECT", "*", "FROM"],
            "my_table_name",
            "",
            &[],
            "",
            &[],
            &[],
            "",
            "",
        );
        assert!(sql.contains("FROM my_table_name"));
    }

    #[test]
    fn test_build_query_with_multiple_sql_parts() {
        let sql = build_query(
            &["SELECT", "id, name, age", "FROM", "INNER JOIN"],
            "users u",
            "ON u.dept_id = d.id",
            &[],
            "",
            &[],
            &[],
            "",
            "",
        );
        assert!(sql.starts_with("SELECT id, name, age FROM INNER JOIN users u"));
        assert!(sql.contains("ON u.dept_id = d.id"));
    }
}
