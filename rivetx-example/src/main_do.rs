use crate::anyhow_tests::anyhow_tests;
use crate::linked_hash_map_tests::linked_hash_map_tests;
use crate::log4_tests;
use crate::moka_tests::moka_tests;
use anyhow::Context;
use rivetx_core::rivetx_string_tests::rivetx_string_tests;
use rivetx_core::spawnx_tests::spawnx_tests;
use rivetx_core::thread_panic::thread_panic;
use rivetx_sql::rivetx_sql_tests::rivetx_sql_tests;

//run --bin rivetx-example
//test --workspace -- --nocapture
//+nightly test --workspace -- --nocapture
//+nightly test --workspace -- --nocapture --test-threads=1

pub async fn do_main() -> anyhow::Result<()> {
    std::panic::set_hook(Box::new(thread_panic));

    let log4_handle = log4_tests::init_file("./conf/log4rs.yaml").here()?;
    log4_tests::http_server(58180, log4_handle).here()?;

    #[cfg(unix)]
    {
        use rivetx_core::linux_limit::set_linux_limit;
        use rivetx_core::linux_limit::Limit;
        set_linux_limit(Limit::default_limit())
            .map_err(|e| anyhow!("Failed to set_linux_limit: {:?}", e))?;
    }

    log::info!("do_main start");

    log::info!("anyhow_tests");
    anyhow_tests().await.here()?;

    log::info!("rivetx_sql_tests");
    rivetx_sql_tests().await.here()?;

    log::info!("spawnx_tests");
    spawnx_tests().await.here()?;

    log::info!("linked_hash_map_tests");
    linked_hash_map_tests().await.here()?;

    log::info!("moka_tests");
    moka_tests().await.here()?;

    log::info!("rivetx_string_tests");
    rivetx_string_tests().await.here()?;

    log::info!("do_main end");

    Ok(())
}
