use std::{collections::HashMap, net::SocketAddr, path::Path, sync::Arc};

use anyhow::{bail, Result};
use async_trait::async_trait;
use futures::lock::Mutex;
use log::error;
use s2n_quic::{connection::StreamAcceptor, provider::datagram::default::Endpoint};

use crate::{
    io::{read_packet, send_request, write_packet},
    msg::{MsgStream, MsgType},
    stream::{new_reader, new_writer},
    Client, Reader, Writer,
};

struct QuicClient {
    handle: s2n_quic::connection::Handle,
    tx_map: Arc<Mutex<HashMap<String, async_channel::Sender<Reader>>>>,
}

#[async_trait]
impl Client for QuicClient {
    async fn open_stream(&mut self, channel: &str) -> Result<Writer> {
        let mut writer = new_writer(self.handle.open_send_stream().await?);
        let msg = MsgStream {
            channel: channel.to_owned(),
        };
        write_packet(&mut writer, msg).await?;
        Ok(writer)
    }

    async fn subscribe(&mut self, channel: &str) -> Result<async_channel::Receiver<Reader>> {
        let mut tx_map = self.tx_map.lock().await;

        if tx_map.contains_key(channel) {
            bail!("This channel has been subscribed.");
        }

        let msg = MsgStream {
            channel: channel.to_owned(),
        };
        let payload = rmp_serde::to_vec_named(&msg)?;
        send_request(&self.handle, MsgType::Subcribe, payload).await?;

        let (tx, rx) = async_channel::unbounded();
        tx_map.insert(channel.to_owned(), tx);
        Ok(rx)
    }

    async fn close(&mut self) -> Result<()> {
        self.handle.close(1u32.into());
        Ok(())
    }
}

async fn run_accept_streams(
    mut acceptor: StreamAcceptor,
    tx_map: Arc<Mutex<HashMap<String, async_channel::Sender<Reader>>>>,
) -> Result<()> {
    while let Some(stream) = acceptor.accept_receive_stream().await? {
        let mut reader: Reader = new_reader(stream);
        let msg: MsgStream = read_packet(&mut reader).await?;
        if let Some(tx) = tx_map.lock().await.get(&msg.channel) {
            tx.send(reader).await?;
        }
    }
    Ok(())
}

pub async fn new_client(
    server_addr: SocketAddr,
    server_name: &str,
    cert: &Path,
) -> Result<Box<dyn Client>> {
    let datagram_provider = Endpoint::builder()
        .with_send_capacity(200)?
        .with_recv_capacity(200)?
        .build()?;

    match s2n_quic::Client::builder()
        .with_tls(cert)?
        .with_io("0.0.0.0:0")?
        .with_datagram(datagram_provider)?
        .start()
    {
        Err(e) => {
            bail!("{}", e)
        }
        Ok(c) => {
            let mut conn = c
                .connect(s2n_quic::client::Connect::new(server_addr).with_server_name(server_name))
                .await?;
            conn.keep_alive(true)?;

            let (handle, acceptor) = conn.split();
            let tx_map = Arc::new(Mutex::new(HashMap::new()));
            let tx_map_cloned = tx_map.clone();
            tokio::spawn(async move {
                if let Err(e) = run_accept_streams(acceptor, tx_map_cloned).await {
                    error!("run_accept_streams: {}", e);
                }
            });

            Ok(Box::new(QuicClient { handle, tx_map }))
        }
    }
}
