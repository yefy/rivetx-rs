use crate::spawnx::{
    defer_async, tokio_batch_add, tokio_batch_close, tokio_batch_flush, tokio_batch_spawn,
    tokio_list_add, tokio_list_close, tokio_list_spawn, tokio_spawn, tokio_timer_spawn,
    tokio_uniq_spawn, TokioUniqSpawn,
};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn unique_name(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}_{}", prefix, nanos)
}

#[tokio::test]
async fn test_defer_async_runs_deferred_items_in_reverse_order() {
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));
    let order_clone = order.clone();

    let result = defer_async(|defer| {
        let order1 = order_clone.clone();
        let order2 = order_clone.clone();
        Box::pin(async move {
            defer.add(move || {
                order1.lock().unwrap().push(1);
                Ok(())
            });
            defer.add_fut(async move {
                order2.lock().unwrap().push(2);
                Ok(())
            });
            order_clone.lock().unwrap().push(3);
            Ok(())
        })
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(*order.lock().unwrap(), vec![3, 2, 1]);
}

#[tokio::test]
async fn test_tokio_spawn_executes_task_without_panic() {
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    tokio_spawn(async move {
        executed_clone.store(true, Ordering::SeqCst);
        Ok(())
    });

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(executed.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_tokio_uniq_spawn_waits_for_task_completion() {
    let name = unique_name("uniq_spawn_test");
    let counter = Arc::new(AtomicI32::new(0));
    let counter_clone = counter.clone();

    tokio_uniq_spawn(&name, true, async move {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        Ok(())
    })
    .await;

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_tokio_batch_spawn_add_flush_close_triggers_worker() {
    let name = unique_name("batch_spawn_test");
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    tokio_batch_spawn(&name, 2, Duration::from_millis(1), move |datas| {
        let executed = executed_clone.clone();
        async move {
            if !datas.is_empty() {
                executed.store(true, Ordering::SeqCst);
            }
            Ok(())
        }
    });

    tokio_batch_add(&name, Box::new(1_i32)).await;
    tokio_batch_add(&name, Box::new(2_i32)).await;
    tokio_batch_flush(&name).await;
    tokio_batch_close(&name).await;

    let result = tokio::time::timeout(Duration::from_secs(1), async {
        while !executed.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_tokio_list_spawn_add_close_runs_consumer() {
    let name = unique_name("list_spawn_test");
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    tokio_list_spawn(&name, 2, move |data| {
        let executed = executed_clone.clone();
        async move {
            if let Some(data) = data {
                let value = data.downcast::<i32>().unwrap();
                assert_eq!(*value, 1);
                executed.store(true, Ordering::SeqCst);
            }
            Ok(())
        }
    });

    tokio_list_add(&name, Box::new(1_i32)).await;
    tokio_list_close(&name).await;

    let result = tokio::time::timeout(Duration::from_secs(1), async {
        while !executed.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_tokio_timer_spawn_quits_after_first_call() {
    let name = unique_name("timer_spawn_test");
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = executed.clone();

    tokio_timer_spawn(&name, false, Duration::from_millis(1), move || {
        let executed = executed_clone.clone();
        async move {
            executed.store(true, Ordering::SeqCst);
            (true, Ok(()))
        }
    });

    let result = tokio::time::timeout(Duration::from_secs(1), async {
        while !executed.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_tokio_uniq_spawn_direct_context_run() {
    let spawn = TokioUniqSpawn::new();
    spawn.run();

    let counter = Arc::new(AtomicI32::new(0));
    let counter_clone = counter.clone();
    spawn
        .add(true, async move {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await;

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}
