use crate::task_group::TaskGroup;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Short shutdown timeout used in most tests (seconds).
const SHUTDOWN_TIMEOUT: u64 = 1;

// ────────── Construction ──────────

#[tokio::test]
async fn test_task_group_new() {
    let tg = TaskGroup::new();
    assert!(!tg.is_quit());
    assert_eq!(tg.count(), 0);
}

// ────────── Quit ──────────

#[tokio::test]
async fn test_task_group_quit_sets_flag() {
    let tg = TaskGroup::new();
    tg.quit(false, SHUTDOWN_TIMEOUT).await.unwrap();
    assert!(tg.is_quit());
}

#[tokio::test]
async fn test_task_group_quit_idempotent() {
    let tg = TaskGroup::new();
    tg.quit(false, SHUTDOWN_TIMEOUT).await.unwrap();
    assert!(tg.is_quit());
    tg.quit(false, SHUTDOWN_TIMEOUT).await.unwrap();
    assert!(tg.is_quit());
}

#[tokio::test]
async fn test_task_group_quit_fast_shutdown_waits_for_tasks() {
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

    tg.quit(true, SHUTDOWN_TIMEOUT).await.unwrap();
    assert!(counter.load(Ordering::Relaxed));
    assert!(tg.is_quit());
}

#[tokio::test]
async fn test_task_group_quit_without_fast_shutdown() {
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

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(task_started.load(Ordering::Relaxed));

    tg.quit(false, SHUTDOWN_TIMEOUT).await.unwrap();
    assert!(tg.is_quit());
}

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

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert!(task_started.load(Ordering::Relaxed));

    tg.quit(true, SHUTDOWN_TIMEOUT).await.unwrap();
    assert!(task_completed.load(Ordering::Relaxed));
    assert!(tg.is_quit());
}

// ────────── send / subscribe ──────────

#[tokio::test]
async fn test_task_group_send_broadcast() {
    let tg = TaskGroup::new();
    let mut rx = tg.subscribe();

    tg.send().await.unwrap();

    assert_eq!(rx.recv().await.unwrap(), true);
}

#[tokio::test]
async fn test_task_group_quit_fast_shutdown_broadcasts() {
    let tg = TaskGroup::new();
    let mut rx = tg.subscribe();

    tg.quit(true, SHUTDOWN_TIMEOUT).await.unwrap();

    assert_eq!(rx.recv().await.unwrap(), true);
}

#[tokio::test]
async fn test_task_group_subscribe_multiple_receivers() {
    let tg = TaskGroup::new();
    let mut rx1 = tg.subscribe();
    let mut rx2 = tg.subscribe();

    tg.send().await.unwrap();

    assert_eq!(rx1.recv().await.unwrap(), true);
    assert_eq!(rx2.recv().await.unwrap(), true);
}

#[tokio::test]
async fn test_task_group_subscribe_with_clones() {
    let tg1 = TaskGroup::new();
    let tg2 = tg1.clone();
    let mut rx = tg1.subscribe();

    tg2.send().await.unwrap();
    assert_eq!(rx.recv().await.unwrap(), true);
}

#[tokio::test]
async fn test_task_group_subscribe_after_send() {
    let tg = TaskGroup::new();
    let mut rx = tg.subscribe();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        tg.send().await.unwrap();
    });

    assert_eq!(rx.recv().await.unwrap(), true);
}

#[tokio::test]
async fn test_task_group_subscribe_guard_then_send() {
    let tg1 = TaskGroup::new();
    let tg2 = tg1.clone();

    let guard = tg2.guard_add();
    let tg_send = tg2.clone();
    tokio::spawn(async move {
        let _guard = guard;
        let mut rx = tg2.subscribe();
        tg_send.send().await.unwrap();
        assert_eq!(rx.recv().await.unwrap(), true);
    });

    tg1.wait().await.unwrap();
}

// ────────── add / add_num / done / wait ──────────

#[tokio::test]
async fn test_task_group_add_done_wait() {
    let tg = TaskGroup::new();
    tg.add();
    assert_eq!(tg.count(), 1);
    tg.done();
    assert_eq!(tg.count(), 0);
    tg.wait().await.unwrap();
}

#[tokio::test]
async fn test_task_group_add_num() {
    let tg = TaskGroup::new();
    tg.add_num(3);
    assert_eq!(tg.count(), 3);
    tg.done();
    tg.done();
    tg.done();
    assert_eq!(tg.count(), 0);
    tg.wait().await.unwrap();
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
    let total: i32 = 10;
    let counter = Arc::new(std::sync::atomic::AtomicI32::new(0));

    tg.add_num(total as usize);
    for _ in 0..total {
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

#[tokio::test]
async fn test_task_group_wait_empty() {
    let tg = TaskGroup::new();
    tg.wait().await.unwrap();
}

#[tokio::test]
async fn test_task_group_wait_after_quit() {
    let tg = TaskGroup::new();
    tg.add();
    tg.done();
    tg.quit(false, SHUTDOWN_TIMEOUT).await.unwrap();
    tg.wait().await.unwrap();
}

// ────────── guard_add ──────────

#[tokio::test]
async fn test_task_group_guard_add_auto_done_on_drop() {
    let tg = TaskGroup::new();
    {
        let _guard = tg.guard_add();
    }
    tg.wait().await.unwrap();
}

#[tokio::test]
async fn test_task_group_multiple_guards() {
    let tg = TaskGroup::new();
    let guard1 = tg.guard_add();
    let guard2 = tg.guard_add();
    drop(guard1);
    drop(guard2);
    tg.wait().await.unwrap();
}

// ────────── clone ──────────

#[tokio::test]
async fn test_task_group_clone_shares_state() {
    let tg1 = TaskGroup::new();
    let tg2 = tg1.clone();

    assert!(!tg1.is_quit());
    assert!(!tg2.is_quit());

    tg1.quit(false, SHUTDOWN_TIMEOUT).await.unwrap();
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

    tg1.wait().await.unwrap();
    tg2.wait().await.unwrap();
}

// ────────── done_error ──────────

#[tokio::test]
async fn test_task_group_done_error_surfaces_on_wait() {
    let tg = TaskGroup::new();
    tg.add();
    tg.done_error(anyhow::anyhow!("task failed"));
    let result = tg.wait().await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("task failed"));
}
