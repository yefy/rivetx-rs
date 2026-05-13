use rivetx_example::main_do::do_main;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    scopeguard::defer! {
        log::logger().flush();
    };
    if let Err(e) = do_main().await {
        log::error!("err: {:#?}", e);
    } else {
        log::info!("Success!");
    }
    Ok(())
}
