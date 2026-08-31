//! 不走 wasm。多线程 + 大量 tokio 任务压 `pool.get_conn`，对照 wasm demo 的共用池 / 每请求新建池。
//!
//! ```text
//! cargo test -p rivetx-sql --features native -- --ignored --nocapture get_conn_stress
//! ```
//!
//! 可选：`STRESS_TASKS`（默认 200）、`STRESS_LOOPS`（默认 20）、`STRESS_OS_THREADS`（默认 8）。

use crate::conn::RivetxSql;
use crate::create::{create_database_from_url, url_without_pool_bounds};
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn mysql_url() -> String {
    let url = std::env::var("TEST_RIVETX_MYSQL_URL").unwrap_or_default();
    let url = url.trim();
    assert!(
        !url.is_empty(),
        "TEST_RIVETX_MYSQL_URL must be set, e.g. mysql://user:pass@host:3306/test_db"
    );
    url.to_string()
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(default)
}

fn timeout() -> Duration {
    Duration::from_secs(15)
}

fn is_wsaeinval(err: &str) -> bool {
    err.contains("10022") || err.contains("WSAEINVAL") || err.contains("无效的参数")
}

#[derive(Default)]
struct Counters {
    ok: AtomicU64,
    err: AtomicU64,
    wsaeinval: AtomicU64,
    first_err: Mutex<Option<String>>,
}

impl Counters {
    fn record(&self, r: Result<(), String>) {
        match r {
            Ok(()) => {
                self.ok.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => {
                self.err.fetch_add(1, Ordering::Relaxed);
                if is_wsaeinval(&e) {
                    self.wsaeinval.fetch_add(1, Ordering::Relaxed);
                }
                let mut g = self.first_err.lock().unwrap();
                if g.is_none() {
                    *g = Some(e);
                }
            }
        }
    }

    fn summary(&self, name: &str) -> String {
        let first = self.first_err.lock().unwrap().clone().unwrap_or_default();
        format!(
            "{}: ok={} err={} wsaeinval=10022:{} first_err={}",
            name,
            self.ok.load(Ordering::Relaxed),
            self.err.load(Ordering::Relaxed),
            self.wsaeinval.load(Ordering::Relaxed),
            first
        )
    }

    fn wsaeinval(&self) -> u64 {
        self.wsaeinval.load(Ordering::Relaxed)
    }
}

async fn spawn_all<F, Fut>(n_tasks: usize, work: F) -> Arc<Counters>
where
    F: Fn(usize) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), String>> + Send + 'static,
{
    let counters = Arc::new(Counters::default());
    let work = Arc::new(work);
    let mut joins = Vec::with_capacity(n_tasks);
    for i in 0..n_tasks {
        let counters = counters.clone();
        let work = work.clone();
        joins.push(tokio::spawn(async move {
            counters.record(work(i).await);
        }));
    }
    for j in joins {
        let _ = j.await;
    }
    counters
}

async fn exec_loops(sql: RivetxSql, loops: usize) -> Result<(), String> {
    for _ in 0..loops {
        sql.exec("SELECT 1", &[], timeout())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 一套 multi-thread runtime，很多协程。共用池 vs 每任务新建池。
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore]
async fn get_conn_stress_multi_task() {
    let url = mysql_url();
    let small_url = url_without_pool_bounds(&url);
    let tasks = env_usize("STRESS_TASKS", 200);
    let loops = env_usize("STRESS_LOOPS", 20);

    let shared_small = {
        let sql = RivetxSql::new(&small_url, 8, 64).expect("shared small pool");
        let c = spawn_all(tasks, {
            let sql = sql.clone();
            move |_| exec_loops(sql.clone(), loops)
        })
        .await;
        let _ = sql.disconnect().await;
        c
    };

    let shared_url = {
        let sql = RivetxSql::new(&url, 50, 100).expect("shared url pool");
        let c = spawn_all(tasks, {
            let sql = sql.clone();
            move |_| exec_loops(sql.clone(), loops)
        })
        .await;
        let _ = sql.disconnect().await;
        c
    };

    let create_db = spawn_all(tasks, {
        let url = url.clone();
        move |_| {
            let url = url.clone();
            async move {
                for _ in 0..loops {
                    create_database_from_url(&url, timeout())
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Ok(())
            }
        }
    })
    .await;

    // 小池 new+disconnect：create_database 同款，Windows 上应过。
    let new_pool_small = spawn_all(tasks, {
        let url = small_url.clone();
        move |_| {
            let url = url.clone();
            async move {
                for _ in 0..loops {
                    let sql = RivetxSql::new(&url, 1, 2).map_err(|e| e.to_string())?;
                    let r = sql.exec("SELECT 1", &[], timeout()).await;
                    let _ = sql.disconnect().await;
                    r.map_err(|e| e.to_string())?;
                }
                Ok(())
            }
        }
    })
    .await;

    let lines = [
        shared_small.summary("shared_small_pool"),
        shared_url.summary("shared_url_pool"),
        create_db.summary("create_database_per_task"),
        new_pool_small.summary("new_pool_small_per_task"),
    ];
    for line in &lines {
        eprintln!("{line}");
    }
    let wsa = shared_small.wsaeinval()
        + shared_url.wsaeinval()
        + create_db.wsaeinval()
        + new_pool_small.wsaeinval();
    assert_eq!(
        wsa,
        0,
        "os error 10022 in native mysql (no wasm)\n{}",
        lines.join("\n")
    );
}

/// URL 带 pool_min=100 时，并发「每任务 new 池 + get_conn + disconnect」。
/// Windows 上 mysql_async/ConnectEx 会偶发 10022；Linux 一般没有。这不是 wasm fiber。
/// 生产应共用一个长期池，不要这么写。
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore]
async fn get_conn_stress_fat_new_pool() {
    let url = mysql_url();
    let tasks = env_usize("STRESS_TASKS", 200);
    let loops = env_usize("STRESS_LOOPS", 20);
    let c = spawn_all(tasks, {
        let url = url.clone();
        move |_| {
            let url = url.clone();
            async move {
                for _ in 0..loops {
                    let sql = RivetxSql::new(&url, 50, 100).map_err(|e| e.to_string())?;
                    let r = sql.exec("SELECT 1", &[], timeout()).await;
                    let _ = sql.disconnect().await;
                    r.map_err(|e| e.to_string())?;
                }
                Ok(())
            }
        }
    })
    .await;
    let line = c.summary("new_pool_fat_url_per_task");
    eprintln!("{line}");
    if c.wsaeinval() > 0 {
        eprintln!(
            "known Windows mysql_async: concurrent new pool (pool_min from URL) + disconnect => 10022"
        );
    }
}

/// 多 OS 线程，每线程独立 runtime + 一批协程（不共享 reactor）。
#[test]
#[ignore]
fn get_conn_stress_os_threads() {
    let url = mysql_url();
    let small_url = url_without_pool_bounds(&url);
    let os_threads = env_usize("STRESS_OS_THREADS", 8);
    let tasks = env_usize("STRESS_TASKS", 32);
    let loops = env_usize("STRESS_LOOPS", 10);
    let counters = Arc::new(Counters::default());

    std::thread::scope(|scope| {
        for _ in 0..os_threads {
            let url = url.clone();
            let small_url = small_url.clone();
            let counters = counters.clone();
            scope.spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .expect("os-thread runtime");
                rt.block_on(async move {
                    let sql = RivetxSql::new(&small_url, 4, 32).expect("thread pool");
                    let c1 = spawn_all(tasks, {
                        let sql = sql.clone();
                        move |_| exec_loops(sql.clone(), loops)
                    })
                    .await;
                    let _ = sql.disconnect().await;

                    let c2 = spawn_all(tasks, {
                        let url = url.clone();
                        move |_| {
                            let url = url.clone();
                            async move {
                                let sql = RivetxSql::new(&url, 2, 8).map_err(|e| e.to_string())?;
                                let r = sql.exec("SELECT 1", &[], timeout()).await;
                                let _ = sql.disconnect().await;
                                r.map(|_| ()).map_err(|e| e.to_string())
                            }
                        }
                    })
                    .await;

                    for src in [c1, c2] {
                        counters
                            .ok
                            .fetch_add(src.ok.load(Ordering::Relaxed), Ordering::Relaxed);
                        counters
                            .err
                            .fetch_add(src.err.load(Ordering::Relaxed), Ordering::Relaxed);
                        counters
                            .wsaeinval
                            .fetch_add(src.wsaeinval(), Ordering::Relaxed);
                        let mut dst = counters.first_err.lock().unwrap();
                        if dst.is_none() {
                            *dst = src.first_err.lock().unwrap().clone();
                        }
                    }
                });
            });
        }
    });

    let line = counters.summary("os_threads");
    eprintln!("{line}");
    assert_eq!(
        counters.wsaeinval(),
        0,
        "os error 10022 across OS threads (no wasm)\n{line}"
    );
}
