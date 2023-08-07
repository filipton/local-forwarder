use std::{collections::HashMap, sync::Arc};

use color_eyre::Result;
use tokio::sync::RwLock;

pub struct ChanneledChannel<T> {
    channels: Arc<RwLock<HashMap<u16, (async_channel::Sender<T>, async_channel::Receiver<T>)>>>,
}

#[allow(dead_code)]
impl<T> ChanneledChannel<T> {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_channel(&self, id: &u16) -> Result<()> {
        let (tx, rx) = async_channel::unbounded::<T>();
        self.channels.write().await.insert(*id, (tx, rx));

        Ok(())
    }

    pub async fn remove_channel(&self, id: &u16) -> Result<()> {
        self.channels.write().await.remove(id);

        Ok(())
    }

    pub async fn get_sender(&self, id: &u16) -> Option<async_channel::Sender<T>> {
        let channels = self.channels.read().await;
        channels.get(id).map(|(tx, _)| tx.clone())
    }

    pub async fn get_receiver(&self, id: &u16) -> Option<async_channel::Receiver<T>> {
        let channels = self.channels.read().await;
        channels.get(id).map(|(_, rx)| rx.clone())
    }

    pub fn clone(&self) -> Self {
        Self {
            channels: self.channels.clone(),
        }
    }
}
