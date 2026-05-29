use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct TaskGroupContext {
    await_group: awaitgroup::WaitGroup,
    quit_tx: tokio::sync::broadcast::Sender<bool>,
    is_quit: AtomicBool,
}

#[derive(Clone)]
pub struct TaskGroup {
    data: Arc<TaskGroupContext>,
}

impl TaskGroup {
    pub fn new() -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(10);
        Self {
            data: Arc::new(TaskGroupContext {
                await_group: awaitgroup::WaitGroup::new(),
                quit_tx: tx,
                is_quit: AtomicBool::new(false),
            }),
        }
    }
    pub fn is_quit(&self) -> bool {
        return self.data.is_quit.load(Ordering::Relaxed);
    }

    pub async fn quit(&self, is_wait: bool) {
        if self.data.is_quit.load(Ordering::Relaxed) {
            return;
        }
        self.data.is_quit.store(true, Ordering::Relaxed);
        let _ = self.data.quit_tx.send(true);
        if is_wait {
            if let Err(e) = self.wait().await {
                log::error!("err:{:?}", e);
            }
        }
    }

    pub async fn wait(&self) -> anyhow::Result<()> {
        loop {
            let ret = tokio::time::timeout(
                tokio::time::Duration::from_secs(1),
                self.data.await_group.wait(),
            )
            .await;
            match ret {
                Ok(Ok(())) => return Ok(()),
                Ok(Err(e)) => return Err(anyhow::anyhow!("err:{}", e)),
                Err(_) => {
                    let _ = self.data.quit_tx.send(true);
                }
            }
        }
    }

    pub fn add(&self) {
        self.data.await_group.add()
    }

    pub fn add_num(&self, num: i32) {
        self.data.await_group.add_num(num)
    }

    pub fn guard_add(&self) -> awaitgroup::WaitGroupGuard {
        self.data.await_group.guard_add()
    }

    pub fn done(&self) {
        self.data.await_group.done();
    }

    pub fn set_error(&self, err: anyhow::Error) {
        self.data.await_group.set_error(err)
    }

    pub fn count(&self) -> i32 {
        self.data.await_group.count()
    }

    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<bool> {
        self.data.quit_tx.subscribe()
    }
}
