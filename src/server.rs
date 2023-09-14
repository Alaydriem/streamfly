use std::{collections::HashMap, path::Path, sync::Arc};

use anyhow::{bail, Result};
use futures::{lock::Mutex, AsyncWriteExt};
use log::{debug, error, info};
use s2n_quic::{
    connection::Handle, provider::datagram::default::Endpoint, stream::ReceiveStream, Connection,
};

use crate::{
    io::{read_packet, recv_request, write_packet},
    msg::{MsgOpenStream, MsgSubscribeStream, MsgType},
    stream::{new_reader, new_writer},
    Reader, Writer,
};

pub async fn serve(addr: &str, cert: &Path, key: &Path) -> Result<()> {
    let datagram_provider = Endpoint::builder()
        .with_send_capacity(200)?
        .with_recv_capacity(200)?
        .build()?;

    match s2n_quic::Server::builder()
        .with_tls((cert, key))?
        .with_io(addr)?
        .with_datagram(datagram_provider)?
        .start()
    {
        Err(e) => {
            bail!("{}", e)
        }
        Ok(mut s) => {
            let all_handles = Arc::new(Mutex::new(HashMap::new()));

            while let Some(conn) = s.accept().await {
                let all_handles_cloned = all_handles.clone();
                tokio::spawn(async move {
                    if let Err(e) = process_conn(all_handles_cloned, conn).await {
                        error!("process_conn: {}", e);
                    }
                });
            }

            Ok(())
        }
    }
}

async fn process_conn(
    all_handles: Arc<Mutex<HashMap<String, (String, Handle)>>>,
    conn: Connection,
) -> Result<()> {
    let (handle, mut acceptor) = conn.split();
    let remote_addr = handle.remote_addr()?.to_string();
    info!("connection ++: {}", remote_addr);

    let all_handles_cloned = all_handles.clone();
    tokio::spawn(async move {
        if let Err(e) = recv_datagrams_loop(all_handles_cloned, handle).await {
            error!("recv_datagrams_loop: {}", e);
        }
    });

    while let Ok(Some(stream)) = acceptor.accept_receive_stream().await {
        let all_handles_cloned = all_handles.clone();
        tokio::spawn(async move {
            if let Err(e) = process_recv_stream(all_handles_cloned, stream).await {
                error!("process_recv_stream: {}", e);
            }
        });
    }

    info!("connection --: {}", remote_addr);
    if let Some((channel, _)) = all_handles.lock().await.remove(&remote_addr) {
        info!("subscriber --: [{}], {}", channel, remote_addr);
    }

    Ok(())
}

async fn recv_datagrams_loop(
    all_handles: Arc<Mutex<HashMap<String, (String, Handle)>>>,
    handle: Handle,
) -> Result<()> {
    let remote_addr = handle.remote_addr()?.to_string();
    loop {
        let req = recv_request(&handle).await?;

        match req.msg_type {
            MsgType::Subcribe => {
                let msg: MsgSubscribeStream = rmp_serde::from_slice(&req.payload)?;
                let mut all_handles = all_handles.lock().await;
                info!("subscriber ++: [{}], {}", msg.channel, remote_addr);
                all_handles.insert(remote_addr.to_owned(), (msg.channel, handle.to_owned()));
            }
        }
    }
}

async fn process_recv_stream(
    all_handles: Arc<Mutex<HashMap<String, (String, Handle)>>>,
    stream: ReceiveStream,
) -> Result<()> {
    let remote_addr = stream.connection().remote_addr()?.to_string();
    let mut reader: Reader = new_reader(stream);
    info!("recv_stream ++: {}", remote_addr);

    let msg: MsgOpenStream = read_packet(&mut reader).await?;
    let mut all_handles = all_handles.lock().await;

    let mut tx_list = vec![];
    for (channel, handle) in all_handles.values_mut() {
        if channel == &msg.channel {
            let tx = open_stream(handle, &msg.channel, &msg.stream_id).await?;
            tx_list.push(tx);
        }
    }
    drop(all_handles);

    while let Some(buf) = reader.receive().await? {
        debug!("recv {} bytes from {}", buf.len(), remote_addr);

        for tx in &tx_list {
            tx.send(buf.to_owned().into()).await?;
        }
    }

    for tx in &tx_list {
        tx.close();
    }
    info!("recv_stream --: {}", remote_addr);

    Ok(())
}

async fn open_stream(
    handle: &mut Handle,
    channel: &str,
    stream_id: &str,
) -> Result<async_channel::Sender<Vec<u8>>> {
    let stream = handle.open_send_stream().await?;
    let remote_addr = stream.connection().remote_addr()?.to_string();
    let mut writer: Writer = new_writer(stream);
    info!("send_stream ++: {}", remote_addr);

    let msg = MsgOpenStream {
        channel: channel.to_owned(),
        stream_id: stream_id.to_owned(),
    };
    write_packet(&mut writer, &msg).await?;

    let (tx, rx) = async_channel::unbounded::<Vec<u8>>();
    tokio::spawn(async move {
        while let Ok(buf) = rx.recv().await {
            debug!("send {} bytes to {}", buf.len(), remote_addr);
            writer.write_all(&buf).await?;
        }
        info!("send_stream --: {}", remote_addr);
        anyhow::Ok(())
    });

    Ok(tx)
}
