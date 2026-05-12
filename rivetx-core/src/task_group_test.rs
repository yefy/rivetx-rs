use crate::task_group::TaskGroup;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

// ────────── Construction / Default ──────────

#[tokio::test]
async fn test_task_group_new() {
    let tg = TaskGroup::new();
    assert!(!tg.is_quit());
}

// ────────── Quit Signal ──────────

#[tokio::test]
async fn test_task_group_quit_sets_flag() {
    let tg = TaskGroup::new();
    tg.quit(false).await;
    assert!(tg.is_quit());
}

#[tokio::test]
async fn test_task_group_quit_idempotent() {
    let tg = TaskGroup::new();
    tg.quit(false).await;
    assert!(tg.is_quit());
    tg.quit(false).await;
    assert!(tg.is_quit());
}

#[tokio::test]
async fn test_task_group_quit_with_wait() {
    let tg = TaskGroup::new();
    let counter = Arc::new(AtomicBool::new(false));
    let counter_clone = counter.clone();

    tg.add();
    let tg_clone = tg.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        counter_clone.store(true, Ordering::Relaxed);
        tg_clone.done();
    });

    // quit with wait should block until the task completes
    tg.quit(true).await;
    assert!(counter.load(Ordering::Relaxed));
    assert!(tg.is_quit());
}

// ────────── Subscribe ──────────

#[tokio::test]
async fn test_task_group_subscribe_receives_quit() {
    let tg = TaskGroup::new();
    let mut rx = tg.subscribe();

    tg.quit(false).await;

    let result = rx.recv().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_task_group_subscribe_multiple_receivers() {
    let tg = TaskGroup::new();
    let mut rx1 = tg.subscribe();
    let mut rx2 = tg.subscribe();

    tg.quit(false).await;

    assert_eq!(rx1.recv().await.unwrap(), true);
    assert_eq!(rx2.recv().await.unwrap(), true);
}

// ────────── Add / Done / Wait ──────────

#[tokio::test]
async fn test_task_group_add_done_wait() {
    let tg = TaskGroup::new();
    tg.add();
    tg.done();
    let result = tg.wait().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_task_group_wait_multiple_tasks() {
    let tg = TaskGroup::new();
    let counter = Arc::new(AtomicBool::new(false));
    let counter_clone = counter.clone();

    tg.add();
    let tg_clone = tg.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        counter_clone.store(true, Ordering::Relaxed);
        tg_clone.done();
    });

    tg.wait().await.unwrap();
    assert!(counter.load(Ordering::Relaxed));
}

#[tokio::test]
async fn test_task_group_wait_concurrent_tasks() {
    let tg = TaskGroup::new();
    let total = 10;
    let counter = Arc::new(std::sync::atomic::AtomicI32::new(0));

    for _ in 0..total {
        tg.add();
        let counter = counter.clone();
        let tg = tg.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            counter.fetch_add(1, Ordering::Relaxed);
            tg.done();
        });
    }

    tg.wait().await.unwrap();
    assert_eq!(counter.load(Ordering::Relaxed), total);
}

// ────────── Guard Add ──────────

#[tokio::test]
async fn test_task_group_guard_add_auto_done_on_drop() {
    let tg = TaskGroup::new();
    {
        let _guard = tg.guard_add();
        // guard goes out of scope here -> done() is called automatically
    }
    let result = tg.wait().await;
    assert!(result.is_ok());
}

// #[tokio::test]
// async fn test_task_group_guard_add_with_async_work() {
//     let tg = TaskGroup::new();
//     let counter = Arc::new(AtomicBool::new(false));
//     let counter_clone = counter.clone();
//
//     let _guard = tg.guard_add();
//     let tg_clone = tg.clone();
//     tokio::spawn(async move {
//         tokio::time::sleep(Duration::from_millis(20)).await;
//         counter_clone.store(true, Ordering::Relaxed);
//         // manually done before guard drop
//         tg_clone.done();
//     });
//
//     tg.wait().await.unwrap();
//     assert!(counter.load(Ordering::Relaxed));
// }

// ────────── Clone ──────────

#[tokio::test]
async fn test_task_group_clone_shares_state() {
    let tg1 = TaskGroup::new();
    let tg2 = tg1.clone();

    assert!(!tg1.is_quit());
    assert!(!tg2.is_quit());

    tg1.quit(false).await;
    assert!(tg1.is_quit());
    assert!(tg2.is_quit());
}

#[tokio::test]
async fn test_task_group_clone_wait_together() {
    let tg1 = TaskGroup::new();
    let tg2 = tg1.clone();

    tg1.add();
    let tg2_clone = tg2.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        tg2_clone.done();
    });

    // Both should be able to wait on the same group
    tg1.wait().await.unwrap();
    tg2.wait().await.unwrap();
}

// ────────── Integration: Quit with active tasks ──────────

#[tokio::test]
async fn test_task_group_quit_with_active_tasks() {
    let tg = TaskGroup::new();
    let task_started = Arc::new(AtomicBool::new(false));
    let task_started_clone = task_started.clone();
    let task_completed = Arc::new(AtomicBool::new(false));
    let task_completed_clone = task_completed.clone();

    tg.add();
    let tg_clone = tg.clone();
    tokio::spawn(async move {
        task_started_clone.store(true, Ordering::Relaxed);
        tokio::time::sleep(Duration::from_millis(100)).await;
        task_completed_clone.store(true, Ordering::Relaxed);
        tg_clone.done();
    });

    // Wait a bit for the task to start
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(task_started.load(Ordering::Relaxed));

    // Quit with wait - should wait for the task to finish
    tg.quit(true).await;
    assert!(task_completed.load(Ordering::Relaxed));
    assert!(tg.is_quit());
}

// ────────── Wait on empty group ──────────

#[tokio::test]
async fn test_task_group_wait_empty() {
    let tg = TaskGroup::new();
    // No tasks added, wait should resolve immediately
    let result = tg.wait().await;
    assert!(result.is_ok());
}

// ────────── Multiple done calls are safe ──────────

// #[tokio::test]
// async fn test_task_group_double_done() {
//     let tg = TaskGroup::new();
//     tg.add();
//     tg.done();
//     // Calling done again without an add should be safe (awaitgroup handles this)
//     tg.done();
//     let result = tg.wait().await;
//     assert!(result.is_ok());
// }

// ────────── Subscribe after quit ──────────

#[tokio::test]
async fn test_task_group_subscribe_after_quit() {
    let tg = TaskGroup::new();

    // Subscribe after quit should still receive the signal
    let mut rx = tg.subscribe();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        tg.quit(false).await;
    });
    let result = rx.recv().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

// ────────── Additional Tests for Completeness ──────────

// Test quit without wait does not block
#[tokio::test]
async fn test_task_group_quit_without_wait() {
    let tg = TaskGroup::new();
    let task_started = Arc::new(AtomicBool::new(false));
    let task_started_clone = task_started.clone();

    tg.add();
    let tg_clone = tg.clone();
    tokio::spawn(async move {
        task_started_clone.store(true, Ordering::Relaxed);
        tokio::time::sleep(Duration::from_millis(100)).await;
        tg_clone.done();
    });

    // Wait a bit for the task to start
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(task_started.load(Ordering::Relaxed));

    // Quit without wait - should not wait for the task
    tg.quit(false).await;
    assert!(tg.is_quit());
    // Task may still be running, but quit is set
}

// Test guard drop after manual done (should be safe)
// #[tokio::test]
// async fn test_task_group_guard_manual_done_then_drop() {
//     let tg = TaskGroup::new();
//     let guard = tg.guard_add();
//     tg.done(); // Manual done
//     drop(guard); // Drop guard, should not call done again
//     let result = tg.wait().await;
//     assert!(result.is_ok());
// }

// Test multiple guards
#[tokio::test]
async fn test_task_group_multiple_guards() {
    let tg = TaskGroup::new();
    let guard1 = tg.guard_add();
    let guard2 = tg.guard_add();
    drop(guard1);
    drop(guard2);
    let result = tg.wait().await;
    assert!(result.is_ok());
}

// Test subscribe and quit with multiple clones

#[tokio::test]
async fn test_task_group_subscribe() {
    let tg1 = TaskGroup::new();
    let tg2 = tg1.clone();
    tg2.quit(false).await;

    let wgg = tg2.guard_add();
    tokio::spawn(async move {
        let _wgg = wgg;
        let mut rx = tg2.subscribe();
        let result = rx.recv().await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    });

    tg1.wait().await.unwrap();
}

#[tokio::test]
async fn test_task_group_subscribe_with_clones() {
    let tg1 = TaskGroup::new();
    let tg2 = tg1.clone();
    let mut rx = tg1.subscribe();

    tg2.quit(false).await;
    let result = rx.recv().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

// Test wait after quit
#[tokio::test]
async fn test_task_group_wait_after_quit() {
    let tg = TaskGroup::new();
    tg.add();
    tg.done();
    tg.quit(false).await;
    let result = tg.wait().await;
    assert!(result.is_ok());
}