use anyhow::Context;
use crate::create_tests::test_create;
use crate::delete_tests::test_delete;
use crate::insert_tests::test_insert;
use crate::insert_tests2::test_insert2;
use crate::select_tests::test_select;
use crate::update_tests::test_update;
use crate::util_tests::{
    test_open_rivetx_sql, TestData, TestDataByAs, TestDataByD, TestDataNoExport, Testkey,
};
use crate::FromSqlRow;
use log;

pub async fn rivetx_sql_tests() -> anyhow::Result<()> {
    let meta = TestData::get_struct_meta();
    log::info!("TestData meta:{:#?}", meta);
    let meta = TestDataNoExport::get_struct_meta();
    log::info!("TestDataNoExport meta:{:#?}", meta);
    let meta = TestDataByD::get_struct_meta();
    log::info!("TestDataByD meta:{:#?}", meta);
    let meta = TestDataByAs::get_struct_meta();
    log::info!("TestDataByAs meta:{:#?}", meta);
    let meta = Testkey::get_struct_meta();
    log::info!("Testkey meta:{:#?}", meta);

    log::info!("test_open_rivetx_sql");
    let rivetx_sql = test_open_rivetx_sql().await.here()?;

    log::info!("test_create");
    test_create(&rivetx_sql).await.here()?;

    log::info!("test_insert");
    test_insert(&rivetx_sql).await.here()?;
    test_insert2(&rivetx_sql).await.here()?;

    log::info!("test_select_all");
    test_select(&rivetx_sql).await.here()?;

    log::info!("test_delete");
    test_delete(&rivetx_sql).await.here()?;

    log::info!("test_update");
    test_update(&rivetx_sql).await.here()?;
    Ok(())
}
