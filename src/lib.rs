use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use futures::{AsyncRead, AsyncWrite};

pub type Reader = Pin<Box<dyn AsyncRead + Send + Sync>>;
pub type Writer = Pin<Box<dyn AsyncWrite + Send + Sync>>;

#[async_trait]
pub trait Client: Send + Sync {
    async fn open_stream(&mut self, topic: &str) -> Result<Writer>;

    async fn subscribe(&mut self, topic: &str) -> Result<()>;

    async fn receive_stream(&mut self) -> Result<(String, Reader)>;
}

mod client;
pub use crate::client::connect;

mod server;
pub use crate::server::serve;
