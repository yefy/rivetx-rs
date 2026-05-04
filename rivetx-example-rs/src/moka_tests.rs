use log::info;
use moka::sync::Cache;

pub async fn moka_tests() -> anyhow::Result<()> {
    //let cache = Cache::new(3);
    let cache = Cache::builder()
        .max_capacity(3)
        .eviction_policy(moka::policy::EvictionPolicy::lru())
        .build();

    cache.insert(1, 1);
    cache.insert(2, 2);
    cache.insert(3, 3);
    cache.insert(4, 4);
    let mut len = 0;
    for data in cache.iter() {
        len += 1;
        info!("key:{}, value:{}", data.0, data.1);
    }
    info!("len:{}, entry_count:{}", len, cache.entry_count());

    cache.insert(3, 33);
    let mut len = 0;
    for data in cache.iter() {
        len += 1;
        info!("key:{}, value:{}", data.0, data.1);
    }
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    info!("len:{}, entry_count:{}", len, cache.entry_count());
    cache.run_pending_tasks();
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    let mut len = 0;
    for data in cache.iter() {
        len += 1;
        info!("key:{}, value:{}", data.0, data.1);
    }
    info!("len:{}, entry_count:{}", len, cache.entry_count());

    {
        use std::time::Duration;
        let cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(1))
            .build();

        cache.insert("key1", "value1");
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        let mut len = 0;
        for data in cache.iter() {
            len += 1;
            info!("ttl key:{}, value:{}", data.0, data.1);
        }
        info!("ttl len:{}, entry_count:{}", len, cache.entry_count());

        cache.run_pending_tasks();
        let mut len = 0;
        for data in cache.iter() {
            len += 1;
            info!("ttl key:{}, value:{}", data.0, data.1);
        }
        info!("ttl len:{}, entry_count:{}", len, cache.entry_count());
    }
    Ok(())
}
