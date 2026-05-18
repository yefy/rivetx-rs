use anyhow::Context;
use crate::async_channel::AsyncChannel;
use lazy_static::lazy_static;
use log::error;
use std::future::Future;
use std::mem::swap;
use std::pin::Pin;

type DeferAsyncBoxPinFut = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'static>>;
type DeferAsyncBoxFn = Box<dyn FnOnce() -> anyhow::Result<()> + Send + 'static>;

pub enum DeferAsyncData {
    Fut(DeferAsyncBoxPinFut),
    Fn(DeferAsyncBoxFn),
}

pub struct DeferAsync {
    datas: Vec<DeferAsyncData>,
}

impl DeferAsync {
    pub fn new() -> Self {
        Self {
            datas: Vec::with_capacity(5),
        }
    }

    pub fn add<F>(&mut self, f: F)
    where
        F: FnOnce() -> anyhow::Result<()> + Send + 'static,
    {
        self.datas.push(DeferAsyncData::Fn(Box::new(f)));
    }

    pub fn add_fut<F>(&mut self, fut: F)
    where
        F: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        self.datas.push(DeferAsyncData::Fut(Box::pin(fut)));
    }

    pub async fn run(mut self) {
        self.datas.reverse();
        for data in self.datas {
            match data {
                DeferAsyncData::Fut(fut) => {
                    let ret = fut.await;
                    if let Err(e) = ret {
                        error!("err:{}", e);
                    }
                }
                DeferAsyncData::Fn(f) => {
                    let ret = f();
                    if let Err(e) = ret {
                        error!("err:{}", e);
                    }
                }
            }
        }
    }
}

pub async fn defer_async<F>(fut: F) -> anyhow::Result<()>
where
    F: for<'a> FnOnce(
        &'a mut DeferAsync,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>,
{
    let mut defer = DeferAsync::new();
    let ret = fut(&mut defer).await;
    defer.run().await;
    // if let Err(e) = &ret {
    //     error!("err:{}", e);
    // }
    ret
}

pub fn tokio_spawn<F>(fut: F)
where
    F: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    tokio::spawn(async move {
        let ret = fut.await;
        if let Err(e) = ret {
            error!("err:{:?}", e);
        }
    });
}

type UniqSpawnFut = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'static>>;

pub struct UniqData {
    fut: UniqSpawnFut,
    wait_chan: Option<AsyncChannel<bool>>,
}

pub struct UniqSpawnContext {
    channel: AsyncChannel<UniqData>,
}

impl UniqSpawnContext {
    pub fn new() -> Self {
        Self {
            channel: AsyncChannel::new(1000),
        }
    }
}

#[derive(Clone)]
pub struct TokioUniqSpawn {
    ctx: std::sync::Arc<UniqSpawnContext>,
}

impl TokioUniqSpawn {
    pub fn new() -> Self {
        Self {
            ctx: std::sync::Arc::new(UniqSpawnContext::new()),
        }
    }
    pub fn run(&self) {
        let ctx = self.ctx.clone();
        tokio_spawn(async move {
            loop {
                let ret: Result<UniqData, async_channel::RecvError> = tokio::select!(
                    ret = ctx.channel.rx.recv() => ret,
                );
                let data = ret?;
                let ret = data.fut.await;
                if let Err(e) = ret {
                    error!("err:{:?}", e);
                }
                if data.wait_chan.is_some() {
                    let wait_chan = data.wait_chan.as_ref().unwrap();
                    if let Err(e) = wait_chan.tx.send(true).await {
                        error!("err:{:?}", e);
                    }
                }
            }
        });
    }

    pub async fn add<F>(&self, is_wait: bool, fut: F)
    where
        F: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let wait_chan = if is_wait {
            Some(AsyncChannel::new(0))
        } else {
            None
        };
        let ret = self
            .ctx
            .channel
            .tx
            .send(UniqData {
                fut: Box::pin(fut),
                wait_chan: wait_chan.clone(),
            })
            .await;
        if let Err(e) = ret {
            error!("err:{:?}", e);
        }
        if wait_chan.is_some() {
            let wait_chan = wait_chan.as_ref().unwrap();
            if let Err(e) = wait_chan.rx.recv().await {
                error!("err:{:?}", e);
            }
        }
    }
}

lazy_static! {
    pub static ref UNIQ_SPAWN_MAP: std::sync::RwLock<std::collections::HashMap<String, TokioUniqSpawn>> = {
        let map = std::sync::RwLock::new(std::collections::HashMap::new());
        map
    };
}

pub async fn tokio_uniq_spawn<F>(name: &str, is_wait: bool, fut: F)
where
    F: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let value = UNIQ_SPAWN_MAP.read().unwrap().get(name).cloned();
    let value = if value.is_none() {
        let data = TokioUniqSpawn::new();
        let map = &mut *UNIQ_SPAWN_MAP.write().unwrap();
        let value = map.get(name).cloned();
        if value.is_none() {
            map.insert(name.into(), data.clone());
            data.run();
            data
        } else {
            value.unwrap()
        }
    } else {
        value.unwrap()
    };

    value.add(is_wait, fut).await;
}

const BATCH_DATA_FLUSH: u8 = 0;
const BATCH_DATA_CLOSE: u8 = 1;

pub struct TokioBatchSpawnContext {
    datas: std::sync::Mutex<Vec<Box<dyn std::any::Any + Send>>>,
    chan: AsyncChannel<u8>,
    batch_size: usize,
}

#[derive(Clone)]
pub struct TokioBatchSpawn {
    ctx: std::sync::Arc<TokioBatchSpawnContext>,
}

impl TokioBatchSpawn {
    pub fn new(batch_size: usize) -> Self {
        Self {
            ctx: std::sync::Arc::new(TokioBatchSpawnContext {
                datas: std::sync::Mutex::new(Vec::with_capacity(batch_size * 2)),
                chan: AsyncChannel::new(0),
                batch_size,
            }),
        }
    }

    pub async fn add(&self, data: Box<dyn std::any::Any + Send>) {
        let is_flush = {
            let datas = &mut *self.ctx.datas.lock().unwrap();
            datas.push(data);
            if datas.len() == self.ctx.batch_size {
                true
            } else {
                false
            }
        };
        if is_flush {
            if let Err(e) = self.ctx.chan.tx.send(BATCH_DATA_FLUSH).await {
                error!("err:{:?}", e);
            }
        }
    }

    pub async fn flush(&self) {
        if let Err(e) = self.ctx.chan.tx.send(BATCH_DATA_FLUSH).await {
            error!("err:{:?}", e);
        }
    }

    pub async fn close(&self) {
        if let Err(e) = self.ctx.chan.tx.send(BATCH_DATA_CLOSE).await {
            error!("err:{:?}", e);
        }
    }

    pub fn run<F, Fut>(&self, wait_time: tokio::time::Duration, fut: F)
    where
        F: Fn(Vec<Box<dyn std::any::Any + Send>>) -> Fut + Send + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let ctx = self.ctx.clone();
        tokio_spawn(async move {
            loop {
                let mut flag = BATCH_DATA_FLUSH;
                tokio::select!(
                    ret = ctx.chan.rx.recv() => {
                        flag = ret?;
                    },
                    _ = tokio::time::sleep(wait_time) => {},
                );
                let datas = {
                    let mut datas_ = Vec::with_capacity(ctx.batch_size * 2);
                    let datas = &mut *ctx.datas.lock().unwrap();
                    swap(datas, &mut datas_);
                    datas_
                };
                let ret = fut(datas).await;
                if let Err(e) = ret {
                    error!("err:{:?}", e);
                }

                if flag == BATCH_DATA_CLOSE {
                    return Ok(());
                }
            }
        });
    }
}

lazy_static! {
    pub static ref BATCH_SPAWN_MAP: std::sync::RwLock<std::collections::HashMap<String, TokioBatchSpawn>> = {
        let map = std::sync::RwLock::new(std::collections::HashMap::new());
        map
    };
}

pub async fn tokio_batch_add(name: &str, data: Box<dyn std::any::Any + Send>) {
    let value = BATCH_SPAWN_MAP.read().unwrap().get(name).cloned();
    if value.is_none() {
        error!("err:name:{} tokio_batch_add nil", name);
        return;
    }
    let value = value.unwrap();
    value.add(data).await;
}

pub async fn tokio_batch_flush(name: &str) {
    let value = BATCH_SPAWN_MAP.read().unwrap().get(name).cloned();
    if value.is_none() {
        error!("err:name:{} tokio_batch_flush nil", name);
        return;
    }
    let value = value.unwrap();
    value.flush().await;
}

pub async fn tokio_batch_close(name: &str) {
    let value = BATCH_SPAWN_MAP.read().unwrap().get(name).cloned();
    if value.is_none() {
        error!("err:name:{} tokio_batch_close nil", name);
        return;
    }
    let value = value.unwrap();
    value.close().await;
}

pub fn tokio_batch_spawn<F, Fut>(
    name: &str,
    batch_size: usize,
    wait_time: tokio::time::Duration,
    fut: F,
) where
    F: Fn(Vec<Box<dyn std::any::Any + Send>>) -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let value = BATCH_SPAWN_MAP.read().unwrap().get(name).cloned();
    if value.is_some() {
        return;
    }

    let data = TokioBatchSpawn::new(batch_size);
    let map = &mut *BATCH_SPAWN_MAP.write().unwrap();
    let value = map.get(name).cloned();
    if value.is_some() {
        return;
    }
    map.insert(name.into(), data.clone());
    data.run(wait_time, fut);
}

pub struct TokioTimerSpawnContext {}

#[derive(Clone)]
pub struct TokioTimerSpawn {
    //ctx: std::sync::Arc<TokioTimerSpawnContext>,
}

impl TokioTimerSpawn {
    pub fn new() -> Self {
        Self {
            //ctx: std::sync::Arc::new(TokioTimerSpawnContext {}),
        }
    }

    pub fn run<F, Fut>(&self, is_first_call: bool, wait_time: tokio::time::Duration, fut: F)
    where
        F: Fn() -> Fut + Send + 'static,
        Fut: Future<Output = (bool, anyhow::Result<()>)> + Send + 'static,
    {
        //let ctx = self.ctx.clone();
        tokio_spawn(async move {
            let mut is_first = true;
            let mut interval = tokio::time::interval(wait_time);
            loop {
                if is_first {
                    is_first = false;
                    if !is_first_call {
                        interval.tick().await;
                    }
                } else {
                    interval.tick().await;
                }
                // tokio::select!(
                //     _ = tokio::time::sleep(wait_time) => {},
                // );
                let (is_quit, ret) = fut().await;
                if let Err(e) = ret {
                    error!("err:{:?}", e);
                }

                if is_quit {
                    return Ok(());
                }
            }
        });
    }
}

lazy_static! {
    pub static ref TIMER_SPAWN_MAP: std::sync::RwLock<std::collections::HashMap<String, TokioTimerSpawn>> = {
        let map = std::sync::RwLock::new(std::collections::HashMap::new());
        map
    };
}

pub fn tokio_timer_spawn<F, Fut>(
    name: &str,
    is_first_call: bool,
    wait_time: tokio::time::Duration,
    fut: F,
) where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = (bool, anyhow::Result<()>)> + Send + 'static,
{
    let value = TIMER_SPAWN_MAP.read().unwrap().get(name).cloned();
    if value.is_some() {
        return;
    }

    let data = TokioTimerSpawn::new();
    let map = &mut *TIMER_SPAWN_MAP.write().unwrap();
    let value = map.get(name).cloned();
    if value.is_some() {
        return;
    }
    map.insert(name.into(), data.clone());
    data.run(is_first_call, wait_time, fut);
}

pub struct TokioListSpawnContext {
    chan: AsyncChannel<Option<Box<dyn std::any::Any + Send>>>,
}

#[derive(Clone)]
pub struct TokioListSpawn {
    ctx: std::sync::Arc<TokioListSpawnContext>,
}

impl TokioListSpawn {
    pub fn new(size: usize) -> Self {
        Self {
            ctx: std::sync::Arc::new(TokioListSpawnContext {
                chan: AsyncChannel::new(size),
            }),
        }
    }

    pub async fn add(&self, data: Box<dyn std::any::Any + Send>) {
        if let Err(e) = self.ctx.chan.tx.send(Some(data)).await {
            error!("err:{:?}", e);
        }
    }

    pub async fn close(&self) {
        if let Err(e) = self.ctx.chan.tx.send(None).await {
            error!("err:{:?}", e);
        }
    }

    pub fn run<F, Fut>(&self, fut: F)
    where
        F: Fn(Option<Box<dyn std::any::Any + Send>>) -> Fut + Send + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let ctx = self.ctx.clone();
        tokio_spawn(async move {
            loop {
                let data = ctx.chan.rx.recv().await.here()?;
                let is_data_nil = data.is_none();
                let ret = fut(data).await;
                if let Err(e) = ret {
                    error!("err:{:?}", e);
                }

                if is_data_nil {
                    return Ok(());
                }
            }
        });
    }
}

lazy_static! {
    pub static ref LIST_SPAWN_MAP: std::sync::RwLock<std::collections::HashMap<String, TokioListSpawn>> = {
        let map = std::sync::RwLock::new(std::collections::HashMap::new());
        map
    };
}

pub async fn tokio_list_add(name: &str, data: Box<dyn std::any::Any + Send>) {
    let value = LIST_SPAWN_MAP.read().unwrap().get(name).cloned();
    if value.is_none() {
        error!("err:name:{} tokio_list_add nil", name);
        return;
    }
    let value = value.unwrap();
    value.add(data).await;
}

pub async fn tokio_list_close(name: &str) {
    let value = LIST_SPAWN_MAP.read().unwrap().get(name).cloned();
    if value.is_none() {
        error!("err:name:{} tokio_list_close nil", name);
        return;
    }
    let value = value.unwrap();
    value.close().await;
}

pub fn tokio_list_spawn<F, Fut>(name: &str, size: usize, fut: F)
where
    F: Fn(Option<Box<dyn std::any::Any + Send>>) -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let value = LIST_SPAWN_MAP.read().unwrap().get(name).cloned();
    if value.is_some() {
        return;
    }

    let data = TokioListSpawn::new(size);
    let map = &mut *LIST_SPAWN_MAP.write().unwrap();
    let value = map.get(name).cloned();
    if value.is_some() {
        return;
    }
    map.insert(name.into(), data.clone());
    data.run(fut);
}
