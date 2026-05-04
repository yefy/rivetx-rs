use crate::linked_hash_map_tests::linked_hash_map_tests;
use crate::log4_tests;
use crate::moka_tests::moka_tests;
use rivetx_core_rs::rivetx_string_tests::rivetx_string_tests;
use rivetx_core_rs::spawnx_tests::spawnx_tests;
use rivetx_core_rs::thread_panic::thread_panic;
use rivetx_sql_rs::rivetx_sql_tests::rivetx_sql_tests;

//run --bin rivetx-example-rs
//test --workspace -- --nocapture
//+nightly test --workspace -- --nocapture
pub async fn do_main() -> anyhow::Result<()> {
    std::panic::set_hook(Box::new(thread_panic));

    let log4_handle = log4_tests::init_file("./conf/log4rs.yaml")?;
    log4_tests::http_server(58080, log4_handle)?;

    #[cfg(unix)]
    {
        use rivetx_core_rs::linux_limit::set_linux_limit;
        use rivetx_core_rs::linux_limit::Limit;
        set_linux_limit(Limit::default_limit())
            .map_err(|e| anyhow!("Failed to set_linux_limit: {:?}", e))?;
    }

    log::info!("do_main start");
    rivetx_sql_tests().await?;
    spawnx_tests().await?;
    linked_hash_map_tests().await?;
    moka_tests().await?;
    rivetx_string_tests()?;
    log::info!("do_main end");

    Ok(())
}
