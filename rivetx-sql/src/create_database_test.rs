#[cfg(test)]
mod tests {
    use crate::conn::RivetxSql;
    use crate::create::{
        create_database, create_database_on, generate_create_database_sql,
    };
    use crate::util_tests::{lock_test_db, test_open_rivetx_sql};
    use mysql_async::prelude::Queryable;
    use std::time::Duration;

    const MYSQL_HOST: &str = "mysql://root:Yfygz@389@192.168.192.139:3306";
    const TEMP_DB: &str = "rivetx_create_db_ut";

    fn timeout() -> Duration {
        Duration::from_secs(5)
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
        let sql = format!(
            "DROP DATABASE IF EXISTS `{}`",
            db_name.replace('`', "``")
        );
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

    #[tokio::test]
    async fn test_create_database_rejects_empty_name() {
        let err = create_database(MYSQL_HOST, "", timeout())
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("db name is empty"),
            "err={}",
            err
        );
    }

    #[tokio::test]
    async fn test_create_database_on_rejects_empty_name() {
        let rivetx_sql = RivetxSql::new(MYSQL_HOST, 1, 2).unwrap();
        let err = create_database_on(&rivetx_sql, "", timeout())
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("db name is empty"),
            "err={}",
            err
        );
        rivetx_sql.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_create_database_with_url_without_db() {
        let _guard = lock_test_db();
        create_database(MYSQL_HOST, "test_db", timeout())
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
        create_database(MYSQL_HOST, "test_db", timeout())
            .await
            .unwrap();
        create_database(MYSQL_HOST, "test_db", timeout())
            .await
            .unwrap();

        let rivetx_sql = test_open_rivetx_sql().await.unwrap();
        assert!(schema_exists(&rivetx_sql, "test_db").await);
        rivetx_sql.disconnect().await.unwrap();
    }

    #[tokio::test]
    async fn test_create_database_new_schema_then_drop() {
        let _guard = lock_test_db();
        let setup = RivetxSql::new(MYSQL_HOST, 1, 2).unwrap();
        drop_database(&setup, TEMP_DB).await;

        create_database(MYSQL_HOST, TEMP_DB, timeout())
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
        let setup = RivetxSql::new(MYSQL_HOST, 1, 2).unwrap();
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
