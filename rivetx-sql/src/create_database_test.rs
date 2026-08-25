#[cfg(test)]
mod tests {
    use crate::conn::RivetxSql;
    use crate::create::{
        create_database, create_database_from_url, create_database_on,
        generate_create_database_sql, parse_create_database_url,
    };
    use crate::util_tests::{lock_test_db, test_open_rivetx_sql};
    use mysql_async::prelude::Queryable;
    use std::time::Duration;

    const TEMP_DB: &str = "rivetx_create_db_ut";

    fn mysql_host() -> &'static str {
        static HOST: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        HOST.get_or_init(|| {
            let host = std::env::var("TEST_RIVETX_MYSQL_HOST").unwrap_or_default();
            let host = host.trim();
            assert!(
                !host.is_empty(),
                "TEST_RIVETX_MYSQL_HOST must be set, e.g. mysql://user:pass@host:3306"
            );
            host.to_string()
        })
    }

    fn mysql_url() -> &'static str {
        static URL: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        URL.get_or_init(|| {
            let url = std::env::var("TEST_RIVETX_MYSQL_URL").unwrap_or_default();
            let url = url.trim();
            assert!(
                !url.is_empty(),
                "TEST_RIVETX_MYSQL_URL must be set, e.g. mysql://user:pass@host:3306/test_db"
            );
            url.to_string()
        })
    }

    fn timeout() -> Duration {
        Duration::from_secs(5)
    }

    fn mysql_url_with_query(url: &str, query: &str) -> String {
        if url.contains('?') {
            format!("{}&{}", url, query)
        } else {
            format!("{}?{}", url, query)
        }
    }

    async fn schema_exists(rivetx_sql: &RivetxSql, db_name: &str) -> bool {
        let mut conn = rivetx_sql.conn().await.unwrap();
        let sql = format!(
            "SELECT SCHEMA_NAME FROM INFORMATION_SCHEMA.SCHEMATA WHERE SCHEMA_NAME = '{}'",
            db_name.replace('\'', "''")
        );
        let dbs: Vec<String> = conn.query(sql).await.unwrap();
        !dbs.is_empty()
    }

    async fn drop_database(rivetx_sql: &RivetxSql, db_name: &str) {
        let sql = format!("DROP DATABASE IF EXISTS `{}`", db_name.replace('`', "``"));
        rivetx_sql.exec(&sql, &[], timeout()).await.unwrap();
    }

    #[test]
    fn test_generate_create_database_sql() {
        let sql = generate_create_database_sql("test_db");
        assert_eq!(sql, "CREATE DATABASE IF NOT EXISTS `test_db`;");
    }

    #[test]
    fn test_generate_create_database_sql_escapes_backtick() {
        let sql = generate_create_database_sql("db`x");
        assert_eq!(sql, "CREATE DATABASE IF NOT EXISTS `db``x`;");
    }

    #[test]
    fn test_parse_create_database_url() {
        let (connect_url, db_name) =
            parse_create_database_url("mysql://user:password@localhost:3306/database").unwrap();
        assert_eq!(connect_url, "mysql://user:password@localhost:3306");
        assert_eq!(db_name, "database");
    }

    #[test]
    fn test_parse_create_database_url_with_query() {
        let (connect_url, db_name) = parse_create_database_url(
            "mysql://user:password@localhost:3306/database?pool_max=10&compress=fast",
        )
        .unwrap();
        assert_eq!(
            connect_url,
            "mysql://user:password@localhost:3306?pool_max=10&compress=fast"
        );
        assert_eq!(db_name, "database");
    }

    #[test]
    fn test_parse_create_database_url_rejects_missing_db() {
        let err = parse_create_database_url("mysql://user:password@localhost:3306").unwrap_err();
        assert!(err.to_string().contains("no database name"), "err={}", err);

        let err = parse_create_database_url("mysql://user:password@localhost:3306/").unwrap_err();
        assert!(err.to_string().contains("no database name"), "err={}", err);

        let err = parse_create_database_url(
            "mysql://user:password@localhost:3306/?pool_max=10&compress=fast",
        )
        .unwrap_err();
        assert!(err.to_string().contains("no database name"), "err={}", err);
    }

    #[tokio::test]
    async fn test_create_database_from_url() {
        let _guard = lock_test_db();
        let url = mysql_url();
        let (_, db_name) = parse_create_database_url(url).unwrap();
        create_database_from_url(url, timeout()).await.unwrap();

        let setup = RivetxSql::new(mysql_host(), 1, 2).unwrap();
        assert!(schema_exists(&setup, &db_name).await);
        setup.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_create_database_from_url_with_query() {
        let _guard = lock_test_db();
        let url = mysql_url_with_query(mysql_url(), "compression=fast");
        let (_, db_name) = parse_create_database_url(&url).unwrap();
        create_database_from_url(&url, timeout()).await.unwrap();

        let setup = RivetxSql::new(mysql_host(), 1, 2).unwrap();
        assert!(schema_exists(&setup, &db_name).await);
        setup.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_create_database_from_url_rejects_missing_db() {
        let err = create_database_from_url(mysql_host(), timeout())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("no database name"), "err={}", err);
    }

    #[tokio::test]
    async fn test_create_database_rejects_empty_name() {
        let err = create_database(mysql_host(), "", timeout())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("db name is empty"), "err={}", err);
    }

    #[tokio::test]
    async fn test_create_database_on_rejects_empty_name() {
        let rivetx_sql = RivetxSql::new(mysql_host(), 1, 2).unwrap();
        let err = create_database_on(&rivetx_sql, "", timeout())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("db name is empty"), "err={}", err);
        rivetx_sql.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_create_database_with_url_without_db() {
        let _guard = lock_test_db();
        create_database(mysql_host(), "test_db", timeout())
            .await
            .unwrap();

        let rivetx_sql = test_open_rivetx_sql().await.unwrap();
        assert!(schema_exists(&rivetx_sql, "test_db").await);
        rivetx_sql.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_create_database_on_if_not_exists() {
        let _guard = lock_test_db();
        let rivetx_sql = test_open_rivetx_sql().await.unwrap();
        create_database_on(&rivetx_sql, "test_db", timeout())
            .await
            .unwrap();
        assert!(schema_exists(&rivetx_sql, "test_db").await);
        rivetx_sql.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_create_database_idempotent() {
        let _guard = lock_test_db();
        create_database(mysql_host(), "test_db", timeout())
            .await
            .unwrap();
        create_database(mysql_host(), "test_db", timeout())
            .await
            .unwrap();

        let rivetx_sql = test_open_rivetx_sql().await.unwrap();
        assert!(schema_exists(&rivetx_sql, "test_db").await);
        rivetx_sql.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_create_database_new_schema_then_drop() {
        let _guard = lock_test_db();
        let setup = RivetxSql::new(mysql_host(), 1, 2).unwrap();
        drop_database(&setup, TEMP_DB).await;

        create_database(mysql_host(), TEMP_DB, timeout())
            .await
            .unwrap();
        assert!(schema_exists(&setup, TEMP_DB).await);

        drop_database(&setup, TEMP_DB).await;
        assert!(!schema_exists(&setup, TEMP_DB).await);
        setup.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_create_database_on_new_schema_then_drop() {
        let _guard = lock_test_db();
        let setup = RivetxSql::new(mysql_host(), 1, 2).unwrap();
        drop_database(&setup, TEMP_DB).await;

        create_database_on(&setup, TEMP_DB, timeout())
            .await
            .unwrap();
        assert!(schema_exists(&setup, TEMP_DB).await);

        drop_database(&setup, TEMP_DB).await;
        assert!(!schema_exists(&setup, TEMP_DB).await);
        setup.disconnect().await.unwrap();
    }
}
