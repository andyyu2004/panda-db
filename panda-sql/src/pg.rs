mod protocol;

use self::protocol::{BackendMessage, FrontendMessage, PgCodec};
use super::*;
use crate::pg::protocol::ReadyForQueryStatus;
use bytes::Bytes;
use futures::{SinkExt, TryStreamExt};
use tokio_util::codec::Decoder;

#[derive(Clone)]
pub(super) struct PgServer {
    engine: Arc<PandaEngine>,
}

pub(super) struct PgSession {
    session: PandaSession,
}

impl PgServer {
    pub fn new(engine: Arc<PandaEngine>) -> Self {
        Self { engine }
    }

    fn new_session(&self) -> PgSession {
        PgSession { session: Arc::clone(&self.engine).new_session() }
    }

    // NOTE for handling errors from tokio::spawn. Maybe just have a top level select loop somewhere that keeps polling the spawn handles?
    pub(crate) async fn handle_pg_connections(
        self,
        pg_addr: impl ToSocketAddrs,
    ) -> PandaResult<()> {
        let listener = TcpListener::bind(pg_addr).await?;
        println!("PandaSQL listening on `{}`", listener.local_addr()?);
        let _handle = tokio::spawn(async move {
            loop {
                let (socket, _) = listener.accept().await?;
                tokio::spawn(self.new_session().handle_connection(socket));
            }
            #[allow(unreachable_code)]
            Ok::<_, PandaError>(())
        });
        Ok(())
    }
}

impl PgSession {
    async fn handle_connection(self, socket: tokio::net::TcpStream) {
        let session = self.session;
        let mut messages = PgCodec::default().framed(socket);
        let result: PandaResult<()> = try {
            while let Some(msg) = messages.try_next().await? {
                match msg {
                    FrontendMessage::StartupMessage { .. } => {
                        messages.send(BackendMessage::AuthenticationOk).await?;
                        messages
                            .send(BackendMessage::ParameterStatus(
                                "client_encoding".to_owned(),
                                "utf-8".to_owned(),
                            ))
                            .await?;
                        messages
                            .send(BackendMessage::ReadyForQuery(ReadyForQueryStatus::Idle))
                            .await?;
                    }
                    FrontendMessage::SSLRequest =>
                        messages.send(BackendMessage::Raw(Bytes::from_static(b"N"))).await?,
                    FrontendMessage::Query(query) => {
                        dbg!(&query);
                        session.query(&query).await?;
                        messages.send(BackendMessage::EmptyQueryResponse).await?;
                        messages
                            .send(BackendMessage::ReadyForQuery(ReadyForQueryStatus::Idle))
                            .await?;
                    }
                    FrontendMessage::Terminate => break,
                }
            }
        };

        if let Err(_err) = result {
            todo!("send error message back")
        }
    }
}
