pub struct AsyncChannel<T> {
    pub tx: async_channel::Sender<T>,
    pub rx: async_channel::Receiver<T>,
}

impl<T> AsyncChannel<T> {
    pub fn new(cap: usize) -> Self {
        let (tx, rx) = if cap > 0 {
            async_channel::bounded(cap)
        } else {
            async_channel::unbounded()
        };
        Self { tx, rx }
    }
}

impl<T> Clone for AsyncChannel<T> {
    fn clone(&self) -> AsyncChannel<T> {
        Self {
            tx: self.tx.clone(),
            rx: self.rx.clone(),
        }
    }
}
