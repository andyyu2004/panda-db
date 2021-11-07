use std::time::Duration;

use super::*;
use panda_db::DEFAULT_LISTEN_ADDR;
use tokio::sync::OnceCell;
use tokio_postgres::NoTls;

static ENGINE: OnceCell<PandaEngine> = OnceCell::const_new();

async fn engine() -> PandaResult<&'static PandaEngine> {
    ENGINE.get_or_try_init(|| PandaEngine::new(DEFAULT_LISTEN_ADDR, DEFAULT_PG_ADDR, vec![])).await
}

async fn pg_client() -> PandaResult<tokio_postgres::Client> {
    let (client, conn) = tokio_postgres::connect("host=localhost port=26630", NoTls).await?;
    tokio::spawn(conn);
    Ok(client)
}

#[tokio::test]
async fn test_connect_to_pg_server() -> PandaResult<()> {
    let _engine = engine().await?;
    let _client = tokio::time::timeout(Duration::from_secs(1), pg_client()).await??;
    Ok(())
}
