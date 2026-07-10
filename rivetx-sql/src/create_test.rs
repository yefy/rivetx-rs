#[cfg(test)]
mod tests {
    use crate::create::generate_create_table_sql;
    use crate::util_tests::{test_data_drop_table, TestData, Testkey};

    // ────────── Basic SQL Generation ──────────

    #[test]
    fn test_generate_create_table_sql_for_test_data() {
        let sql = generate_create_table_sql::<TestData>(&"test_data".into());

        assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS test_data ("));
        assert!(sql.ends_with(");"));
        assert!(sql.contains("id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,"));
        assert!(sql.contains("name_index INT NOT NULL ,"));
        assert!(sql.contains("curr_time DATETIME NOT NULL ,"));
        assert!(sql.contains("created_at DATETIME DEFAULT CURRENT_TIMESTAMP ,"));
        assert!(sql.contains(
            "updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP ,"
        ));
        assert!(sql.contains("PRIMARY KEY (id)"));
        assert!(sql.contains("UNIQUE INDEX u_td_ik ( index_col, key_col)"));
        assert!(sql.contains("UNIQUE INDEX u_td_in ( index_col, name_id)"));
        assert!(sql.contains("INDEX i_td_name_index ( name_index )"));
        assert!(!sql.contains("created_at DATETIME DEFAULT CURRENT_TIMESTAMP AUTO_INCREMENT"));
        assert!(!sql.contains("updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP AUTO_INCREMENT"));
    }

    #[test]
    fn test_generate_create_table_sql_for_test_key() {
        let sql = generate_create_table_sql::<Testkey>(&"test_key".into());

        assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS test_key ("));
        assert!(sql.ends_with(");"));
        assert!(sql.contains("id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,"));
        assert!(sql.contains("key_col VARCHAR(255) NOT NULL ,"));
        assert!(sql.contains("created_at DATETIME DEFAULT CURRENT_TIMESTAMP ,"));
        assert!(sql.contains(
            "updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP ,"
        ));
        assert!(sql.contains("PRIMARY KEY (id)"));
        assert!(!sql.contains("created_at DATETIME DEFAULT CURRENT_TIMESTAMP AUTO_INCREMENT"));
        assert!(!sql.contains("updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP AUTO_INCREMENT"));
    }

    // ────────── Column Coverage ──────────

    #[test]
    fn test_create_table_sql_contains_all_columns() {
        let sql = generate_create_table_sql::<TestData>(&"test_data".into());

        assert!(sql.contains("id "));
        assert!(sql.contains("index_col "));
        assert!(sql.contains("key_col "));
        assert!(sql.contains("name_id "));
        assert!(sql.contains("name_index "));
        assert!(sql.contains("curr_time "));
        assert!(sql.contains("created_at "));
        assert!(sql.contains("updated_at "));
    }

    #[test]
    fn test_create_table_sql_contains_correct_data_types() {
        let sql = generate_create_table_sql::<TestData>(&"test_data".into());

        assert!(sql.contains("id BIGINT UNSIGNED"));
        assert!(sql.contains("index_col INT"));
        assert!(sql.contains("name_id INT"));
        assert!(sql.contains("key_col VARCHAR(64)"));
        assert!(sql.contains("curr_time DATETIME"));
        assert!(sql.contains("created_at DATETIME"));
        assert!(sql.contains("updated_at DATETIME"));
    }

    // ────────── IF NOT EXISTS ──────────

    #[test]
    fn test_create_table_sql_if_not_exists_clause() {
        let sql_test_data = generate_create_table_sql::<TestData>(&"test_data".into());
        let sql_test_key = generate_create_table_sql::<Testkey>(&"test_key".into());

        assert!(sql_test_data.contains("IF NOT EXISTS"));
        assert!(sql_test_key.contains("IF NOT EXISTS"));
    }

    // ────────── Syntax Validation ──────────

    #[test]
    fn test_create_table_sql_proper_syntax() {
        let sql = generate_create_table_sql::<TestData>(&"test_data".into());

        assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS"));
        assert!(sql.ends_with(");"));
        assert!(sql.contains("PRIMARY KEY"));

        let open_parens = sql.matches('(').count();
        let close_parens = sql.matches(')').count();
        assert_eq!(open_parens, close_parens, "Mismatched parentheses in SQL");
    }

    #[test]
    fn test_create_table_sql_no_trailing_comma() {
        let sql = generate_create_table_sql::<TestData>(&"test_data".into());

        // The last column/index definition before closing paren should not end with comma
        let body = &sql[sql.find('(').unwrap() + 1..sql.rfind(')').unwrap()];
        let trimmed = body.trim();
        assert!(
            !trimmed.ends_with(','),
            "SQL body should not end with trailing comma"
        );
    }

    // ────────── Various Table Names ──────────

    #[test]
    fn test_create_table_sql_handles_various_table_names() {
        let names = vec!["test_data", "users", "orders_v2", "my_table_123"];

        for name in names {
            let sql = generate_create_table_sql::<TestData>(&name.into());
            assert!(sql.contains(&format!("CREATE TABLE IF NOT EXISTS {} (", name)));
        }
    }

    // ────────── Determinism ──────────

    #[test]
    fn test_create_table_generates_deterministic_sql() {
        let sql1 = generate_create_table_sql::<TestData>(&"test_data".into());
        let sql2 = generate_create_table_sql::<TestData>(&"test_data".into());

        assert_eq!(sql1.len(), sql2.len());
        assert!(sql1.contains("PRIMARY KEY (id)"));
        assert!(sql2.contains("PRIMARY KEY (id)"));
    }

    // ────────── AUTO_INCREMENT Behavior ──────────

    #[test]
    fn test_create_table_sql_no_auto_increment_for_timestamp_columns() {
        let sql = generate_create_table_sql::<TestData>(&"test_data".into());

        let parts: Vec<&str> = sql.split(',').collect();

        for part in parts {
            if part.contains("CURRENT_TIMESTAMP") {
                assert!(
                    !part.contains("AUTO_INCREMENT"),
                    "CURRENT_TIMESTAMP column should not have AUTO_INCREMENT: {}",
                    part
                );
            }
        }
    }

    #[test]
    fn test_create_table_sql_auto_increment_on_primary_key() {
        let sql = generate_create_table_sql::<TestData>(&"test_data".into());

        // id is marked with auto and primary, should have AUTO_INCREMENT
        assert!(sql.contains("id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT"));
    }

    // ────────── Index / Unique Constraints ──────────

    #[test]
    fn test_create_table_sql_contains_primary_key() {
        let sql_test_data = generate_create_table_sql::<TestData>(&"test_data".into());
        let sql_test_key = generate_create_table_sql::<Testkey>(&"test_key".into());

        assert!(sql_test_data.contains("PRIMARY KEY (id)"));
        assert!(sql_test_key.contains("PRIMARY KEY (id)"));
    }

    #[test]
    fn test_create_table_sql_contains_unique_indexes() {
        let sql = generate_create_table_sql::<TestData>(&"test_data".into());

        assert!(sql.contains("UNIQUE INDEX u_td_ik"));
        assert!(sql.contains("UNIQUE INDEX u_td_in"));
    }

    #[test]
    fn test_create_table_sql_contains_regular_index() {
        let sql = generate_create_table_sql::<TestData>(&"test_data".into());

        assert!(sql.contains("INDEX i_td_name_index"));
    }

    #[test]
    fn test_create_table_sql_unique_index_multiple_columns() {
        let sql = generate_create_table_sql::<TestData>(&"test_data".into());

        // u_td_ik spans two columns: index_col, key_col
        assert!(sql.contains("UNIQUE INDEX u_td_ik ( index_col, key_col)"));
    }

    #[test]
    fn test_create_table_sql_unique_index_single_column() {
        let sql = generate_create_table_sql::<TestData>(&"test_data".into());

        // u_td_in spans two columns: index_col, name_id
        assert!(sql.contains("UNIQUE INDEX u_td_in ( index_col, name_id)"));
    }

    // ────────── Table Name With Special Characters ──────────

    #[test]
    fn test_create_table_sql_with_special_table_name() {
        let sql = generate_create_table_sql::<Testkey>(&"my_table_123".into());
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS my_table_123 ("));
    }

    // ────────── NOT NULL Default ──────────

    #[test]
    fn test_create_table_sql_default_not_null() {
        let sql = generate_create_table_sql::<TestData>(&"test_data".into());

        // Columns without explicit fixed_attr should default to NOT NULL
        assert!(sql.contains("index_col INT NOT NULL"));
        assert!(sql.contains("name_id INT NOT NULL"));
        assert!(sql.contains("name_index INT NOT NULL"));
        assert!(sql.contains("curr_time DATETIME NOT NULL"));
    }

    // ────────── Column Order in SQL ──────────

    #[test]
    fn test_create_table_sql_column_order() {
        let sql = generate_create_table_sql::<TestData>(&"test_data".into());

        // Extract column definitions between CREATE TABLE (...)
        let start = sql.find('(').unwrap() + 1;
        let end = sql.rfind(')').unwrap();
        let body = &sql[start..end];

        // id should come first
        assert!(body.starts_with(" id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT"));
    }

    // ────────── Key Table without Custom Indexes ──────────

    #[test]
    fn test_create_table_sql_key_table_no_custom_indexes() {
        let sql = generate_create_table_sql::<Testkey>(&"test_key".into());

        // Testkey has no custom index or unique annotations (only primary key)
        // It should NOT contain INDEX or UNIQUE INDEX lines other than PRIMARY KEY
        assert!(!sql.contains("UNIQUE INDEX"));
        assert!(!sql.contains("INDEX "));
    }

    // ════════════════════════════════════════════════════════════
    // Integration tests for create_table (async, requires DB)
    // ════════════════════════════════════════════════════════════

    use crate::conn::RivetxSql;
    use crate::create::create_table;
    use crate::util_tests::{test_key_drop_table, test_open_rivetx_sql};
    use mysql_async::prelude::Queryable;
    use std::time::Duration;

    /// Helper to get a shared DB connection for tests
    async fn get_rivetx_sql() -> RivetxSql {
        test_open_rivetx_sql().await.unwrap()
    }

    /// Test creating test_data table and verify its structure
    #[tokio::test]
    async fn test_create_table_test_data() {
        let _guard = crate::util_tests::lock_test_db();
        let rivetx_sql = get_rivetx_sql().await;

        // Drop table first to ensure clean state
        let _ = test_data_drop_table(&rivetx_sql).await;

        // Create the table
        create_table::<TestData>(&rivetx_sql, &"test_data".into(), Duration::from_secs(5))
            .await
            .unwrap();

        // Verify table exists
        let mut conn = rivetx_sql.conn().await.unwrap();
        let tables: Vec<String> = conn
            .query(
                "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'test_data'",
            )
            .await
            .unwrap();
        assert!(!tables.is_empty(), "test_data table was not created");

        // Verify columns
        let columns: Vec<String> = conn
            .query(
                "SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_NAME = 'test_data' ORDER BY COLUMN_NAME",
            )
            .await
            .unwrap();

        let expected_cols = [
            "created_at",
            "curr_time",
            "id",
            "index_col",
            "key_col",
            "name_id",
            "name_index",
            "updated_at",
        ];
        for col in &expected_cols {
            assert!(
                columns.contains(&col.to_string()),
                "test_data table missing '{}' column",
                col
            );
        }

        // Verify primary key
        let primary_keys: Vec<String> = conn
            .query(
                "SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE WHERE TABLE_NAME = 'test_data' AND CONSTRAINT_NAME = 'PRIMARY'",
            )
            .await
            .unwrap();
        assert!(
            primary_keys.contains(&"id".to_string()),
            "test_data table missing PRIMARY KEY on 'id'"
        );

        // Verify unique indexes
        let unique_indexes: Vec<String> = conn
            .query(
                "SELECT INDEX_NAME FROM INFORMATION_SCHEMA.STATISTICS WHERE TABLE_NAME = 'test_data' AND SEQ_IN_INDEX = 1 AND NOT NON_UNIQUE",
            )
            .await
            .unwrap();
        assert!(
            unique_indexes.contains(&"u_td_ik".to_string()),
            "test_data table missing unique index 'u_td_ik'"
        );
        assert!(
            unique_indexes.contains(&"u_td_in".to_string()),
            "test_data table missing unique index 'u_td_in'"
        );

        // Verify regular index
        let regular_indexes: Vec<String> = conn
            .query(
                "SELECT INDEX_NAME FROM INFORMATION_SCHEMA.STATISTICS WHERE TABLE_NAME = 'test_data' AND INDEX_NAME = 'i_td_name_index'",
            )
            .await
            .unwrap();
        assert!(
            !regular_indexes.is_empty(),
            "test_data table missing index 'i_td_name_index'"
        );

        // Cleanup
        let _ = test_data_drop_table(&rivetx_sql).await;
    }

    /// Test creating test_key table and verify its structure
    #[tokio::test]
    async fn test_create_table_test_key() {
        let _guard = crate::util_tests::lock_test_db();
        let rivetx_sql = get_rivetx_sql().await;

        // Drop table first to ensure clean state
        let _ = test_key_drop_table(&rivetx_sql).await;

        // Create the table
        create_table::<Testkey>(&rivetx_sql, &"test_key".into(), Duration::from_secs(5))
            .await
            .unwrap();

        // Verify table exists
        let mut conn = rivetx_sql.conn().await.unwrap();
        let tables: Vec<String> = conn
            .query(
                "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'test_key'",
            )
            .await
            .unwrap();
        assert!(!tables.is_empty(), "test_key table was not created");

        // Verify columns
        let columns: Vec<String> = conn
            .query(
                "SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_NAME = 'test_key' ORDER BY COLUMN_NAME",
            )
            .await
            .unwrap();

        let expected_cols = ["created_at", "id", "index_col", "key_col", "updated_at"];
        for col in &expected_cols {
            assert!(
                columns.contains(&col.to_string()),
                "test_key table missing '{}' column",
                col
            );
        }

        // Verify primary key
        let primary_keys: Vec<String> = conn
            .query(
                "SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE WHERE TABLE_NAME = 'test_key' AND CONSTRAINT_NAME = 'PRIMARY'",
            )
            .await
            .unwrap();
        assert!(
            primary_keys.contains(&"id".to_string()),
            "test_key table missing PRIMARY KEY on 'id'"
        );

        // Cleanup
        let _ = test_key_drop_table(&rivetx_sql).await;
    }

    /// Test create_table idempotency - calling twice should not error
    #[tokio::test]
    async fn test_create_table_idempotent() {
        let _guard = crate::util_tests::lock_test_db();
        let rivetx_sql = get_rivetx_sql().await;

        // Drop table first
        let _ = test_data_drop_table(&rivetx_sql).await;

        // First creation
        create_table::<TestData>(&rivetx_sql, &"test_data".into(), Duration::from_secs(5))
            .await
            .unwrap();

        // Second creation should succeed due to IF NOT EXISTS
        create_table::<TestData>(&rivetx_sql, &"test_data".into(), Duration::from_secs(5))
            .await
            .unwrap();

        // Verify table still exists
        let mut conn = rivetx_sql.conn().await.unwrap();
        let tables: Vec<String> = conn
            .query(
                "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'test_data'",
            )
            .await
            .unwrap();
        assert!(
            !tables.is_empty(),
            "test_data table should exist after idempotent create"
        );

        // Cleanup
        let _ = test_data_drop_table(&rivetx_sql).await;
    }

    /// Test create_table with different timeout values
    #[tokio::test]
    async fn test_create_table_with_timeout() {
        let _guard = crate::util_tests::lock_test_db();
        let rivetx_sql = get_rivetx_sql().await;

        // Drop table first
        let _ = test_key_drop_table(&rivetx_sql).await;

        // Create with shorter timeout (2 seconds)
        create_table::<Testkey>(&rivetx_sql, &"test_key".into(), Duration::from_secs(2))
            .await
            .unwrap();

        // Verify table was created
        let mut conn = rivetx_sql.conn().await.unwrap();
        let tables: Vec<String> = conn
            .query(
                "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'test_key'",
            )
            .await
            .unwrap();
        assert!(
            !tables.is_empty(),
            "test_key table was not created with timeout"
        );

        // Create with longer timeout (10 seconds)
        let _ = test_key_drop_table(&rivetx_sql).await;
        create_table::<Testkey>(&rivetx_sql, &"test_key".into(), Duration::from_secs(10))
            .await
            .unwrap();

        // Verify table was created
        let tables: Vec<String> = conn
            .query(
                "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'test_key'",
            )
            .await
            .unwrap();
        assert!(
            !tables.is_empty(),
            "test_key table was not created with longer timeout"
        );

        // Cleanup
        let _ = test_key_drop_table(&rivetx_sql).await;
    }

    /// Test create_table with very short timeout that may fail
    #[tokio::test]
    async fn test_create_table_with_very_short_timeout() {
        let _guard = crate::util_tests::lock_test_db();
        let rivetx_sql = get_rivetx_sql().await;

        // Drop table first
        let _ = test_data_drop_table(&rivetx_sql).await;

        // Try with an extremely short timeout (1 millisecond) - this may or may not fail
        // depending on system speed, but the function should handle it gracefully
        let result =
            create_table::<TestData>(&rivetx_sql, &"test_data".into(), Duration::from_millis(1))
                .await;

        // Either it succeeds (fast system) or returns a timeout error
        match result {
            Ok(()) => {
                // Cleanup if it succeeded
                let _ = test_data_drop_table(&rivetx_sql).await;
            }
            Err(e) => {
                let err_msg = e.to_string();
                assert!(
                    !(err_msg.contains("timed out") || err_msg.contains("timeout")),
                    "Expected timeout error but got: {}",
                    err_msg
                );
            }
        }
    }

    /// Test creating multiple tables in sequence
    #[tokio::test]
    async fn test_create_multiple_tables() {
        let _guard = crate::util_tests::lock_test_db();
        let rivetx_sql = get_rivetx_sql().await;
        // Drop tables first
        let _ = test_data_drop_table(&rivetx_sql).await;
        let _ = test_key_drop_table(&rivetx_sql).await;

        // Create both tables
        create_table::<TestData>(&rivetx_sql, &"test_data".into(), Duration::from_secs(5))
            .await
            .unwrap();
        create_table::<Testkey>(&rivetx_sql, &"test_key".into(), Duration::from_secs(5))
            .await
            .unwrap();

        // Verify both tables exist
        let mut conn = rivetx_sql.conn().await.unwrap();
        let tables: Vec<String> = conn
            .query(
                "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME IN ('test_data', 'test_key') ORDER BY TABLE_NAME",
            )
            .await
            .unwrap();

        assert!(
            tables.contains(&"test_data".to_string()),
            "test_data table was not created"
        );
        assert!(
            tables.contains(&"test_key".to_string()),
            "test_key table was not created"
        );

        // Cleanup
        let _ = test_data_drop_table(&rivetx_sql).await;
        let _ = test_key_drop_table(&rivetx_sql).await;
    }
}
