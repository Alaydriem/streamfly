//! # StreamFly
//!
//! A stream-oriented Pub/Sub framework

use anyhow::Result;
use async_trait::async_trait;

mod client;
mod io;
mod msg;
mod server;
mod stream;
pub mod certificate;

pub use crate::client::new_client;
pub use crate::server::serve;
pub use crate::stream::{ Reader, Writer };

use bytes::Bytes;
#[async_trait]
pub trait Mutator: Send + Sync {
    async fn mutate(&self, data: &[u8]) -> anyhow::Result<Bytes, ()>;
}

#[async_trait]
pub trait Client: Send + Sync {
    /// Open a stream according to the channel
    ///
    /// # Examples
    ///
    /// ```
    /// let (stream_id, mut writer) = client.open_stream(CHANNEL).await?;
    ///
    /// writer.write_all(b"Hello, Streamfly!").await?;
    /// ```
    async fn open_stream(&mut self, channel: &str) -> Result<(String, Writer)>;

    /// Subscribe streams
    ///
    /// # Examples
    ///
    /// ```
    /// let rx = client.subscribe(CHANNEL).await?;
    ///
    /// loop {
    ///     let (_, mut reader) = rx.recv().await?;
    ///
    ///     tokio::spawn(async move { tokio::io::copy(&mut reader, &mut tokio::io::stdout()).await });
    /// }
    /// ```
    async fn subscribe(
        &mut self,
        channel: &str
    ) -> Result<async_channel::Receiver<(String, Reader)>>;

    /// Close the client
    async fn close(&mut self) -> Result<()>;
}
