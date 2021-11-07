mod protocol;

use crate::pg::protocol::ReadyForQueryStatus;

use self::protocol::{BackendMessage, FrontendMessage, PgCodec};
use bytes::Bytes;
use futures::{SinkExt, TryStreamExt};
use postgres_protocol::message::backend::{Message, ReadyForQueryBody};
use tokio_util::codec::Decoder;

use super::*;

// NOTE for handling errors from tokio::spawn. Maybe just have a top level select loop somewhere that keeps polling the spawn handles?
pub(crate) async fn handle_pg_connections(pg_addr: impl ToSocketAddrs) -> PandaResult<PgServer> {
    let listener = TcpListener::bind(pg_addr).await?;
    println!("PandaSQL listening on `{}`", listener.local_addr()?);
    let _handle = tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await?;
            tokio::spawn(handle_connection(socket));
        }
        #[allow(unreachable_code)]
        Ok::<_, PandaError>(())
    });
    Ok(PgServer {})
}

async fn handle_connection(socket: tokio::net::TcpStream) {
    handle_connection_inner(socket).await.unwrap()
}

async fn handle_connection_inner(socket: tokio::net::TcpStream) -> PandaResult<()> {
    let mut messages = PgCodec::default().framed(socket);
    loop {
        match messages.try_next().await? {
            Some(msg) => match dbg!(msg) {
                FrontendMessage::StartupMessage { .. } => {
                    messages.send(BackendMessage::AuthenticationOk).await?;
                    messages.send(BackendMessage::ReadyForQuery(ReadyForQueryStatus::Idle)).await?;
                }
                FrontendMessage::SSLRequest =>
                    messages.send(BackendMessage::Raw(Bytes::from_static(b"N"))).await?,
            },
            None => break,
        }
    }
    Ok(())
    // loop {
    //     let mut buf = [0u8; 4];
    //     socket.read_exact(&mut buf).await?;
    //     let n = u32::from_be_bytes(buf) as usize;
    //     let mut buf = vec![0u8; n - 4];
    //     socket.read_exact(&mut buf).await?;
    //     dbg!(String::from_utf8_lossy(&buf));

    //     // AuthentiationOK
    //     socket.write_all(&[b'R', 0, 0, 0, 8, 0, 0, 0, 0]).await?;
    //     // ReadyForQuery
    //     socket.write_all(&[b'Z', 0, 0, 0, 5, b'I']).await?;
    //     dbg!("sent");
    // }
}
