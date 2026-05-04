use rivetx_example_rs::main_do::do_main;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    scopeguard::defer! {
        log::logger().flush();
    };
    if let Err(e) = do_main().await {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        log::error!("err: {:?}", e);
    } else {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        log::info!("Success!");
    }
    Ok(())
}
