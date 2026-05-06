use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct WaitGroupContextContext {
    await_group: awaitgroup::WaitGroup,
    quit_tx: tokio::sync::broadcast::Sender<bool>,
    is_quit: AtomicBool,
}

#[derive(Clone)]
pub struct WaitGroupContext {
    data: Arc<WaitGroupContextContext>,
}

impl WaitGroupContext {
    pub fn new() -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(10);
        Self {
            data: Arc::new(WaitGroupContextContext {
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
        self.data
            .await_group
            .wait()
            .await
            .map_err(|e| anyhow::anyhow!("err:{}", e))?;
        Ok(())
    }

    pub fn add(&self) -> awaitgroup::WorkerInner {
        self.data.await_group.worker().add()
    }
    pub fn done(&self, worker: awaitgroup::WorkerInner) {
        worker.done()
    }

    pub fn quit_chan(&self) -> tokio::sync::broadcast::Receiver<bool> {
        self.data.quit_tx.subscribe()
    }
}
