use std::{collections::HashMap, net::SocketAddr, path::Path, sync::Arc};

use anyhow::{bail, Result};
use async_channel::{unbounded, Receiver, Sender};
use async_trait::async_trait;
use futures::lock::Mutex;
use log::{error, info};
use nanoid::nanoid;
use s2n_quic::{connection::StreamAcceptor, provider::datagram::default::Endpoint};

use crate::{
    io::{read_packet, send_request, write_packet},
    msg::{MsgOpenStream, MsgSubscribeStream, MsgType},
    stream::{new_reader, new_writer},
    Client, Reader, Writer,
};

struct QuicClient {
    handle: s2n_quic::connection::Handle,
    tx_map: Arc<Mutex<HashMap<String, Sender<(String, Reader)>>>>,
}

#[async_trait]
impl Client for QuicClient {
    async fn open_stream(&mut self, channel: &str) -> Result<(String, Writer)> {
        let mut writer = new_writer(self.handle.open_send_stream().await?);
        let msg = MsgOpenStream {
            channel: channel.to_owned(),
            stream_id: nanoid!(),
        };
        write_packet(&mut writer, &msg).await?;
        Ok((msg.stream_id, writer))
    }

    async fn subscribe(&mut self, channel: &str) -> Result<Receiver<(String, Reader)>> {
        let mut tx_map = self.tx_map.lock().await;

        if tx_map.contains_key(channel) {
            bail!("This channel has been subscribed.");
        }

        let msg = MsgSubscribeStream {
            channel: channel.to_owned(),
        };
        let payload = rmp_serde::to_vec_named(&msg)?;
        send_request(&self.handle, MsgType::Subcribe, payload).await?;

        let (tx, rx) = unbounded();
        tx_map.insert(channel.to_owned(), tx);

        Ok(rx)
    }

    async fn close(&mut self) -> Result<()> {
        self.handle.close(1u32.into());
        self.tx_map.lock().await.clear();
        Ok(())
    }
}

async fn run_accept_streams(
    mut acceptor: StreamAcceptor,
    tx_map: &Arc<Mutex<HashMap<String, async_channel::Sender<(String, Reader)>>>>,
) -> Result<()> {
    while let Some(stream) = acceptor.accept_receive_stream().await? {
        let mut reader: Reader = new_reader(stream);
        let msg: MsgOpenStream = read_packet(&mut reader).await?;
        let mut all_tx = tx_map.lock().await;
        if let Some(tx) = all_tx.get(&msg.channel) {
            if tx.send((msg.stream_id, reader)).await.is_err() {
                tx.close();
                all_tx.remove(&msg.channel);
            }
        }
    }
    Ok(())
}

pub async fn new_client(
    server_addr: SocketAddr,
    server_name: &str,
    cert: &Path,
) -> Result<Box<dyn Client>> {
    match s2n_quic::Client::builder()
        .with_tls(cert)?
        .with_io("0.0.0.0:0")?
        .with_datagram(
            Endpoint::builder()
                .with_send_capacity(200)?
                .with_recv_capacity(200)?
                .build()?,
        )?
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
            info!("connected to {}", server_addr);

            let (handle, acceptor) = conn.split();
            let tx_map = Arc::new(Mutex::new(HashMap::new()));
            let tx_map_cloned = tx_map.clone();
            let handle_cloned = handle.clone();
            tokio::spawn(async move {
                if let Err(e) = run_accept_streams(acceptor, &tx_map_cloned).await {
                    error!("run_accept_streams: {}", e);
                }
                info!("disconnected");
                handle_cloned.close(1_u32.into());
                tx_map_cloned.lock().await.clear();
            });

            Ok(Box::new(QuicClient { handle, tx_map }))
        }
    }
}
