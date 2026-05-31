#![allow(deprecated)]

use anyhow::Context;
use crate::spawnx::{
    defer_async, tokio_batch_add, tokio_batch_close, tokio_batch_flush, tokio_batch_spawn,
    tokio_list_add, tokio_list_close, tokio_list_spawn, tokio_spawn, tokio_timer_spawn,
    tokio_uniq_spawn,
};
use crate::task_group::TaskGroup;
use lazy_static::lazy_static;
use log::info;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering};
use std::sync::Arc;

lazy_static! {
    pub static ref WAIT_ALL: std::sync::Arc<TaskGroup> = std::sync::Arc::new(TaskGroup::new());
}

lazy_static! {
    pub static ref WAIT_BATCH: std::sync::Arc<TaskGroup> = std::sync::Arc::new(TaskGroup::new());
}

lazy_static! {
    pub static ref WAIT_UNIQ: std::sync::Arc<TaskGroup> = std::sync::Arc::new(TaskGroup::new());
}

lazy_static! {
    pub static ref WAIT_LIST: std::sync::Arc<TaskGroup> = std::sync::Arc::new(TaskGroup::new());
}

lazy_static! {
    pub static ref WAIT_TIMER: std::sync::Arc<TaskGroup> = std::sync::Arc::new(TaskGroup::new());
}

pub struct Data {
    pub n: i32,
    pub t: i64,
    pub worker: Option<awaitgroup::WaitGroupGuard>,
}

const IS_OPEN_BATCH: bool = true;
const IS_OPEN_UNIQ: bool = true;
const IS_OPEN_LIST: bool = true;
const IS_OPEN_TIMER: bool = true;

const BATCH_SPAWN_MAX: i32 = 1000;
lazy_static! {
    pub static ref BATCH_SPAWN_NUM: std::sync::Arc<AtomicI32> =
        std::sync::Arc::new(AtomicI32::new(0));
}

const UNIQ_SPAWN_MAX: i32 = 1000;
lazy_static! {
    pub static ref UNIQ_SPAWN_NUM: std::sync::Arc<AtomicI32> =
        std::sync::Arc::new(AtomicI32::new(0));
}

const LIST_SPAWN_MAX: i32 = 1000;
lazy_static! {
    pub static ref LIST_SPAWN_NUM: std::sync::Arc<AtomicI32> =
        std::sync::Arc::new(AtomicI32::new(0));
}

const TIMER_SPAWN_MAX: i32 = 100;
const TIMER_SPAWN_SLEEP: u64 = 10;
lazy_static! {
    pub static ref TIMER_SPAWN_NUM: std::sync::Arc<AtomicI32> =
        std::sync::Arc::new(AtomicI32::new(0));
}

lazy_static! {
    pub static ref BATCH_SPAWN_LOG: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::with_capacity(1000000)));
}

lazy_static! {
    pub static ref UNIQ_SPAWN_LOG: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::with_capacity(1000000)));
}

lazy_static! {
    pub static ref list_spawn_log: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::with_capacity(1000000)));
}

lazy_static! {
    pub static ref LIST_SPAWN_LOG: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::with_capacity(1000000)));
}

pub async fn test_tokio_batch_spawn() {
    let name = "test1";
    tokio_batch_spawn(
        name,
        100,
        tokio::time::Duration::from_millis(1000),
        move |datas| async move {
            let datas_len = datas.len();
            if datas_len <= 0 {
                return Ok(());
            }
            let data_end_time = chrono::Local::now().timestamp_nanos();
            let data_stat_time = {
                let last_data = datas.get(datas.len() - 1);
                if last_data.is_none() {
                    0
                } else {
                    let last_data = last_data.unwrap();
                    let last_data = last_data.downcast_ref::<Data>();
                    if last_data.is_none() {
                        0
                    } else {
                        last_data.unwrap().t
                    }
                }
            };
            for data in datas {
                let data = data.downcast::<Data>();
                if data.is_err() {
                    continue;
                }
                let mut data = *data.unwrap();

                let worker = data.worker.take();
                scopeguard::defer! {
                    if worker.is_some() {
                        drop(worker.unwrap());
                    }
                };

                async {
                    let batch_num = BATCH_SPAWN_NUM.fetch_add(1, Ordering::Relaxed) + 1;
                    let log = format!(
                        "n:{:?}, BATCH_SPAWN_NUM:{}, nanos:{}",
                        data.n,
                        batch_num,
                        (data_end_time - data_stat_time) / (datas_len as i64)
                    );
                    {
                        BATCH_SPAWN_LOG.lock().unwrap().push(log);
                    }
                }
                .await;
            }
            Ok(())
        },
    );

    for index in 0..BATCH_SPAWN_MAX {
        let data = Data {
            n: index,
            t: chrono::Local::now().timestamp_nanos(),
            worker: Some(WAIT_BATCH.guard_add()),
        };
        tokio_batch_add(name, Box::new(data)).await;
    }
    tokio_batch_flush(name).await;
    tokio_batch_close(name).await;
}

pub async fn test_tokio_uniq_spawn() {
    let name = "test1";
    for index in 0..UNIQ_SPAWN_MAX {
        let data = Data {
            n: index,
            t: chrono::Local::now().timestamp_nanos(),
            worker: None,
        };
        WAIT_UNIQ.add();
        tokio_uniq_spawn(name, true, async move {
            scopeguard::defer! {
                WAIT_UNIQ.done();
            };
            let uniq_num = UNIQ_SPAWN_NUM.fetch_add(1, Ordering::Relaxed) + 1;

            let log = format!(
                "n:{}, UNIQ_SPAWN_NUM:{}, nanos:{}",
                data.n,
                uniq_num,
                chrono::Local::now().timestamp_nanos() - data.t
            );
            {
                UNIQ_SPAWN_LOG.lock().unwrap().push(log);
            }
            return Ok(());
        })
        .await;
    }
}

pub async fn test_tokio_list_spawn() {
    let name = "test1";
    tokio_list_spawn(name, 100, move |data| async move {
        if data.is_none() {
            return Ok(());
        }
        let data = data.unwrap();
        let data = data.downcast::<Data>();
        if data.is_err() {
            return Ok(());
        }
        let mut data = *data.unwrap();

        let worker = data.worker.take();
        scopeguard::defer! {
            if worker.is_some() {
                drop(worker.unwrap());
            }
        };

        let data_end_time = chrono::Local::now().timestamp_nanos();
        let data_stat_time = data.t;

        async {
            let list_num = LIST_SPAWN_NUM.fetch_add(1, Ordering::Relaxed) + 1;
            let log = format!(
                "n:{:?}, LIST_SPAWN_NUM:{}, nanos:{}",
                data.n,
                list_num,
                data_end_time - data_stat_time
            );
            {
                list_spawn_log.lock().unwrap().push(log);
            }
        }
        .await;
        Ok(())
    });

    for index in 0..LIST_SPAWN_MAX {
        let data = Data {
            n: index,
            t: chrono::Local::now().timestamp_nanos(),
            worker: Some(WAIT_LIST.guard_add()),
        };
        tokio_list_add(name, Box::new(data)).await;
    }
    tokio_list_close(name).await;
}

pub async fn test_tokio_timer_spawn() {
    let name = "test1";
    let mut workers = VecDeque::with_capacity(TIMER_SPAWN_MAX as usize);
    for _ in 0..TIMER_SPAWN_MAX {
        let worker = WAIT_TIMER.guard_add();
        workers.push_back(worker);
    }
    let workers = std::sync::Arc::new(std::sync::Mutex::new(workers));
    tokio_timer_spawn(
        name,
        true,
        tokio::time::Duration::from_millis(TIMER_SPAWN_SLEEP),
        move || {
            let workers = workers.clone();
            async move {
                let worker = { workers.lock().unwrap().pop_front() };
                scopeguard::defer! {
                    if worker.is_some() {
                        drop(worker.unwrap());
                    }
                };
                let data = Data {
                    n: TIMER_SPAWN_NUM.load(Ordering::Relaxed),
                    t: chrono::Local::now().timestamp_nanos(),
                    worker: None,
                };
                let timer_num = TIMER_SPAWN_NUM.fetch_add(1, Ordering::Relaxed) + 1;
                let log = format!("n:{:?}, timer_spawn_num:{}", data.n, timer_num);
                {
                    LIST_SPAWN_LOG.lock().unwrap().push(log);
                }
                if timer_num == TIMER_SPAWN_MAX {
                    return (true, Ok(()));
                }
                return (false, Ok(()));
            }
        },
    );
}

pub async fn test_tokio_spawn() {
    let tokio_spawn_num = std::sync::Arc::new(AtomicI32::new(0));
    {
        let tokio_spawn_num = tokio_spawn_num.clone();
        tokio_spawn(async move {
            tokio_spawn_num.fetch_add(1, Ordering::Relaxed);
            info!("tokio_spawn");
            Ok(())
        });
    }
    {
        let tokio_spawn_num = tokio_spawn_num.clone();
        tokio_spawn(async move {
            tokio_spawn_num.fetch_add(1, Ordering::Relaxed);
            info!("tokio_spawn err test");
            return Err(anyhow::anyhow!("tokio_spawn err test")).here()?;
        });
    }
}

pub struct ScopeguardTest {
    pub n: i32,
}

impl ScopeguardTest {
    pub fn new(n: i32) -> Self {
        Self { n }
    }

    pub fn print(&self) {
        info!("ScopeguardTest print m:{}", self.n);
    }

    pub fn print2(&self) {
        info!("ScopeguardTest print2 m:{}", self.n);
    }
}

impl Drop for ScopeguardTest {
    fn drop(&mut self) {
        info!("ScopeguardTest drop m:{}", self.n);
    }
}

pub async fn test_defer_async() -> anyhow::Result<()> {
    let test_scopeguard = ScopeguardTest::new(3);
    let test_scopeguard =
        scopeguard::guard(test_scopeguard, |test_scopeguard| test_scopeguard.print());
    test_scopeguard.print2();

    let scopeguard_num = Arc::new(AtomicI32::new(0));
    let scopeguard_num_defer = scopeguard_num.clone();
    scopeguard::defer! {
        info!("scopeguard_num:{}", scopeguard_num_defer.load(Ordering::Relaxed));
    };
    scopeguard_num.fetch_add(1, Ordering::Relaxed);

    defer_async(move |defer| {
        let scopeguard_num = scopeguard_num.clone();
        Box::pin(async move {
            {
                let scopeguard_num = scopeguard_num.clone();
                defer.add(move || {
                    scopeguard_num.fetch_add(1, Ordering::Relaxed);
                    info!("defer_async defer 000");
                    Ok(())
                });
            }

            {
                let scopeguard_num = scopeguard_num.clone();
                defer.add_fut(async move {
                    scopeguard_num.fetch_add(1, Ordering::Relaxed);
                    info!("defer_async defer 111");
                    Ok(())
                });
            }

            {
                let scopeguard_num = scopeguard_num.clone();
                defer.add_fut(async move {
                    scopeguard_num.fetch_add(1, Ordering::Relaxed);
                    info!("defer_async defer 222");
                    Ok(())
                });
            }

            scopeguard_num.fetch_add(1, Ordering::Relaxed);
            info!("defer_async 111");
            Ok(())
        })
    })
    .await
}

pub async fn spawnx_tests() -> anyhow::Result<()> {
    let start_time = chrono::Local::now().timestamp_millis();
    let batch_spawn_time = std::sync::Arc::new(AtomicI64::new(0));
    let uniq_spawn_time = std::sync::Arc::new(AtomicI64::new(0));
    let list_spawn_time = std::sync::Arc::new(AtomicI64::new(0));
    let timer_spawn_time = std::sync::Arc::new(AtomicI64::new(0));

    if IS_OPEN_BATCH {
        let batch_spawn_time = batch_spawn_time.clone();
        WAIT_ALL.add();
        tokio_spawn(async move {
            defer_async(move |defer| {
                Box::pin(async move {
                    defer.add(|| {
                        WAIT_ALL.done();
                        Ok(())
                    });

                    let start_time = chrono::Local::now().timestamp_millis();
                    test_tokio_batch_spawn().await;
                    WAIT_BATCH.wait().await.here()?;
                    let end_time = chrono::Local::now().timestamp_millis();
                    batch_spawn_time.store(end_time - start_time, Ordering::Relaxed);
                    Ok(())
                })
            })
            .await
        });
    }

    if IS_OPEN_UNIQ {
        let uniq_spawn_time = uniq_spawn_time.clone();
        WAIT_ALL.add();
        tokio_spawn(async move {
            defer_async(move |defer| {
                Box::pin(async move {
                    defer.add(|| {
                        WAIT_ALL.done();
                        Ok(())
                    });

                    let start_time = chrono::Local::now().timestamp_millis();
                    test_tokio_uniq_spawn().await;
                    WAIT_UNIQ.wait().await.here()?;
                    let end_time = chrono::Local::now().timestamp_millis();
                    uniq_spawn_time.store(end_time - start_time, Ordering::Relaxed);
                    Ok(())
                })
            })
            .await
        });
    }

    if IS_OPEN_LIST {
        let list_spawn_time = list_spawn_time.clone();
        WAIT_ALL.add();
        tokio_spawn(async move {
            defer_async(move |defer| {
                Box::pin(async move {
                    defer.add(|| {
                        WAIT_ALL.done();
                        Ok(())
                    });

                    let start_time = chrono::Local::now().timestamp_millis();
                    test_tokio_list_spawn().await;
                    WAIT_LIST.wait().await.here()?;
                    let end_time = chrono::Local::now().timestamp_millis();
                    list_spawn_time.store(end_time - start_time, Ordering::Relaxed);
                    Ok(())
                })
            })
            .await
        });
    }

    if IS_OPEN_TIMER {
        let timer_spawn_time = timer_spawn_time.clone();
        WAIT_ALL.add();
        tokio_spawn(async move {
            defer_async(move |defer| {
                Box::pin(async move {
                    defer.add(|| {
                        WAIT_ALL.done();
                        Ok(())
                    });

                    let start_time = chrono::Local::now().timestamp_millis();
                    test_tokio_timer_spawn().await;
                    WAIT_TIMER.wait().await.here()?;
                    let end_time = chrono::Local::now().timestamp_millis();
                    timer_spawn_time.store(end_time - start_time, Ordering::Relaxed);
                    Ok(())
                })
            })
            .await
        });
    }

    WAIT_ALL.add();
    let mut quit_chan = WAIT_ALL.subscribe();
    tokio_spawn(async move {
        let _ = quit_chan.recv().await;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        WAIT_ALL.done();
        Ok(())
    });

    WAIT_ALL.quit(true, 1).await.here()?;
    let end_time = chrono::Local::now().timestamp_millis();
    let diff_time = end_time - start_time;

    let batch_num = BATCH_SPAWN_NUM.load(Ordering::Relaxed);
    let uniq_num = UNIQ_SPAWN_NUM.load(Ordering::Relaxed);
    let list_num = LIST_SPAWN_NUM.load(Ordering::Relaxed);
    let timer_num = TIMER_SPAWN_NUM.load(Ordering::Relaxed);

    let batch_spawn_time = batch_spawn_time.load(Ordering::Relaxed);
    let uniq_spawn_time = uniq_spawn_time.load(Ordering::Relaxed);
    let list_spawn_time = list_spawn_time.load(Ordering::Relaxed);
    let timer_spawn_time = timer_spawn_time.load(Ordering::Relaxed);

    {
        for log in &*BATCH_SPAWN_LOG.lock().unwrap() {
            info!("{}", log);
        }
    }

    {
        for log in &*UNIQ_SPAWN_LOG.lock().unwrap() {
            info!("{}", log);
        }
    }

    {
        for log in &*list_spawn_log.lock().unwrap() {
            info!("{}", log);
        }
    }

    {
        for log in &*LIST_SPAWN_LOG.lock().unwrap() {
            info!("{}", log);
        }
    }

    info!("batch_spawn_time:{}", batch_spawn_time);
    info!("uniq_spawn_time:{}", uniq_spawn_time);
    info!("list_spawn_time:{}", list_spawn_time);
    info!(
        "timer_spawn_time:{}, avg:{}",
        timer_spawn_time,
        timer_spawn_time / (TIMER_SPAWN_MAX as i64)
    );
    info!("diff_time:{}", diff_time);

    info!(
        "BATCH_SPAWN_NUM:{}, BATCH_SPAWN_MAX:{}",
        batch_num, BATCH_SPAWN_MAX
    );
    if batch_num != BATCH_SPAWN_MAX {
        info!(
            "BATCH_SPAWN_NUMBATCH_SPAWN_NUM:{} != BATCH_SPAWN_MAX:{}",
            batch_num, BATCH_SPAWN_MAX
        );
    }

    info!(
        "UNIQ_SPAWN_NUM:{}, UNIQ_SPAWN_MAX:{}",
        uniq_num, UNIQ_SPAWN_MAX
    );
    if uniq_num != UNIQ_SPAWN_MAX {
        info!(
            "UNIQ_SPAWN_NUM:{} != UNIQ_SPAWN_MAX:{}",
            uniq_num, UNIQ_SPAWN_MAX
        );
    }

    info!(
        "LIST_SPAWN_NUM:{}, LIST_SPAWN_MAX:{}",
        list_num, LIST_SPAWN_MAX
    );
    if list_num != LIST_SPAWN_MAX {
        info!(
            "LIST_SPAWN_NUM:{} != LIST_SPAWN_MAX:{}",
            list_num, LIST_SPAWN_MAX
        );
    }

    info!(
        "timer_spawn_num:{}, TIMER_SPAWN_MAX:{}",
        timer_num, TIMER_SPAWN_MAX
    );
    if timer_num != TIMER_SPAWN_MAX as i32 {
        info!(
            "timer_spawn_num:{} != UNIQ_SPAWN_MAX:{}",
            timer_num, TIMER_SPAWN_MAX
        );
    }

    test_tokio_spawn().await;
    test_defer_async().await.here()?;

    Ok(())
}
