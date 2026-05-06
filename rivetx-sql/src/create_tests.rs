use crate::conn::RivetxSql;
use crate::create::create_table;
use crate::util_tests::*;
use std::time::Duration;

pub async fn test_create(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    test_create_table(rivetx_sql).await?;
    Ok(())
}

pub async fn test_create_table(rivetx_sql: &RivetxSql) -> anyhow::Result<()> {
    test_data_drop_table(rivetx_sql).await?;
    test_key_drop_table(rivetx_sql).await?;

    create_table::<TestData>(rivetx_sql, &"test_data".into(), Duration::from_secs(5)).await?;
    create_table::<Testkey>(rivetx_sql, &"test_key".into(), Duration::from_secs(5)).await?;

    Ok(())
}
