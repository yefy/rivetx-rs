use anyhow::Context;
use crate::conn::RivetxSql;
use crate::create::create_table;
use crate::util_tests::*;
use mysql_async::prelude::Queryable;
use std::time::Duration;

pub async fn test_create(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    test_create_table(rivetx_sql).await.here()?;
    Ok(())
}

pub async fn test_create_table(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    test_data_drop_table(rivetx_sql).await.here()?;
    test_key_drop_table(rivetx_sql).await.here()?;

    create_table::<TestData>(rivetx_sql, &"test_data".into(), Duration::from_secs(5)).await.here()?;
    create_table::<Testkey>(rivetx_sql, &"test_key".into(), Duration::from_secs(5)).await.here()?;

    // Verify tables were created
    test_verify_test_data_table_structure(rivetx_sql).await.here()?;
    test_verify_test_key_table_structure(rivetx_sql).await.here()?;

    Ok(())
}

/// Verify that test_data table was created with correct structure
pub async fn test_verify_test_data_table_structure(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    let mut conn = rivetx_sql.conn().await.here()?;

    // Check if table exists
    let tables: Vec<String> = conn
        .query("SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'test_data'")
        .await.here()?;
    assert!(!tables.is_empty(), "test_data table was not created");

    // Check columns exist
    let columns: Vec<String> = conn
        .query("SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_NAME = 'test_data' ORDER BY COLUMN_NAME")
        .await.here()?;

    assert!(
        columns.contains(&"id".to_string()),
        "test_data table missing 'id' column"
    );
    assert!(
        columns.contains(&"index_col".to_string()),
        "test_data table missing 'index_col' column"
    );
    assert!(
        columns.contains(&"key_col".to_string()),
        "test_data table missing 'key_col' column"
    );
    assert!(
        columns.contains(&"name_id".to_string()),
        "test_data table missing 'name_id' column"
    );
    assert!(
        columns.contains(&"name_index".to_string()),
        "test_data table missing 'name_index' column"
    );
    assert!(
        columns.contains(&"curr_time".to_string()),
        "test_data table missing 'curr_time' column"
    );
    assert!(
        columns.contains(&"created_at".to_string()),
        "test_data table missing 'created_at' column"
    );
    assert!(
        columns.contains(&"updated_at".to_string()),
        "test_data table missing 'updated_at' column"
    );

    // Check primary key
    let primary_keys: Vec<String> = conn
        .query("SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE WHERE TABLE_NAME = 'test_data' AND CONSTRAINT_NAME = 'PRIMARY'")
        .await.here()?;
    assert!(
        primary_keys.contains(&"id".to_string()),
        "test_data table missing PRIMARY KEY on 'id'"
    );

    // Check unique indexes exist
    let unique_indexes: Vec<String> = conn
        .query("SELECT INDEX_NAME FROM INFORMATION_SCHEMA.STATISTICS WHERE TABLE_NAME = 'test_data' AND SEQ_IN_INDEX = 1 AND NOT NON_UNIQUE")
        .await.here()?;
    assert!(
        unique_indexes.contains(&"u_td_ik".to_string()),
        "test_data table missing unique index 'u_td_ik'"
    );
    assert!(
        unique_indexes.contains(&"u_td_in".to_string()),
        "test_data table missing unique index 'u_td_in'"
    );

    // Check regular indexes exist
    let regular_indexes: Vec<String> = conn
        .query("SELECT INDEX_NAME FROM INFORMATION_SCHEMA.STATISTICS WHERE TABLE_NAME = 'test_data' AND INDEX_NAME = 'i_td_name_index'")
        .await.here()?;
    assert!(
        !regular_indexes.is_empty(),
        "test_data table missing index 'i_td_name_index'"
    );

    log::info!("test_data table structure verified successfully");
    Ok(())
}

/// Verify that test_key table was created with correct structure
pub async fn test_verify_test_key_table_structure(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    let mut conn = rivetx_sql.conn().await.here()?;

    // Check if table exists
    let tables: Vec<String> = conn
        .query("SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'test_key'")
        .await.here()?;
    assert!(!tables.is_empty(), "test_key table was not created");

    // Check columns exist
    let columns: Vec<String> = conn
        .query("SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_NAME = 'test_key' ORDER BY COLUMN_NAME")
        .await.here()?;

    assert!(
        columns.contains(&"id".to_string()),
        "test_key table missing 'id' column"
    );
    assert!(
        columns.contains(&"index_col".to_string()),
        "test_key table missing 'index_col' column"
    );
    assert!(
        columns.contains(&"key_col".to_string()),
        "test_key table missing 'key_col' column"
    );
    assert!(
        columns.contains(&"created_at".to_string()),
        "test_key table missing 'created_at' column"
    );
    assert!(
        columns.contains(&"updated_at".to_string()),
        "test_key table missing 'updated_at' column"
    );

    // Check primary key
    let primary_keys: Vec<String> = conn
        .query("SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE WHERE TABLE_NAME = 'test_key' AND CONSTRAINT_NAME = 'PRIMARY'")
        .await.here()?;
    assert!(
        primary_keys.contains(&"id".to_string()),
        "test_key table missing PRIMARY KEY on 'id'"
    );

    log::info!("test_key table structure verified successfully");
    Ok(())
}

/// Test create_table idempotency - calling twice should not error
pub async fn test_create_table_idempotent(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    test_data_drop_table(rivetx_sql).await.here()?;

    // First creation
    create_table::<TestData>(rivetx_sql, &"test_data".into(), Duration::from_secs(5)).await.here()?;

    // Second creation should succeed due to IF NOT EXISTS
    create_table::<TestData>(rivetx_sql, &"test_data".into(), Duration::from_secs(5)).await.here()?;

    log::info!("create_table idempotency test passed");
    Ok(())
}

/// Test create_table with different timeout
pub async fn test_create_table_with_timeout(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    test_key_drop_table(rivetx_sql).await.here()?;

    // Create with shorter timeout
    create_table::<Testkey>(rivetx_sql, &"test_key".into(), Duration::from_secs(2)).await.here()?;

    let mut conn = rivetx_sql.conn().await.here()?;
    let tables: Vec<String> = conn
        .query("SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'test_key'")
        .await.here()?;
    assert!(
        !tables.is_empty(),
        "test_key table was not created with timeout"
    );

    log::info!("create_table with timeout test passed");
    Ok(())
}
