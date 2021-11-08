#![feature(try_blocks)]

#[macro_use]
extern crate anyhow;

mod pg;
mod plan;
mod transaction;

pub use panda_db::PandaResult;
pub use panda_db::DEFAULT_LISTEN_ADDR;

use futures::future;
use panda_db::data::ResultSet;
use panda_db::{PandaError, PandaNetwork, PandaRaft, PandaStorage};
use sqlparser::ast;
use sqlparser::dialect::{Dialect, PostgreSqlDialect};
use sqlparser::parser::{Parser, ParserError};
use std::sync::Arc;
use tokio::net::{TcpListener, ToSocketAddrs};

use self::pg::PgServer;
use self::plan::QueryPlan;
use self::transaction::Transaction;

pub const DEFAULT_PG_ADDR: &str = "127.0.0.1:26630";

pub struct PandaSession {
    engine: Arc<PandaEngine>,
}

const DIALECT: &dyn Dialect = &PostgreSqlDialect {};

impl PandaSession {
    fn parse(&self, query: &str) -> Result<Vec<ast::Statement>, ParserError> {
        Parser::parse_sql(DIALECT, query)
    }

    pub async fn query(&self, query: &str) -> PandaResult<Vec<ResultSet>> {
        let stmts = self.parse(query)?;
        future::try_join_all(stmts.into_iter().map(|stmt| self.execute(stmt))).await
    }

    fn begin(&self) -> Transaction<'_> {
        Transaction::new(&self.engine)
    }

    async fn execute(&self, stmt: ast::Statement) -> PandaResult<ResultSet> {
        let query_plan = self.plan(stmt)?;
        let txn = self.begin();
        query_plan.execute(&txn).await
    }

    pub(crate) fn plan(&self, stmt: ast::Statement) -> PandaResult<QueryPlan> {
        QueryPlan::plan(stmt)
    }
}

pub struct PandaEngine {
    raft: PandaRaft,
}

impl PandaEngine {
    pub async fn new<A>(
        addr: A,
        pg_addr: A,
        join_addrs: impl IntoIterator<Item = A>,
    ) -> PandaResult<Arc<Self>>
    where
        A: ToSocketAddrs + Send + 'static,
    {
        const CLUSTER_NAME: String = String::new();
        let config = async_raft::Config::build(CLUSTER_NAME).validate()?;
        // TODO properly assign node_id according to requirements
        let node_id = 0;
        let network = PandaNetwork::bind(addr, join_addrs).await?;
        let storage = PandaStorage::new(node_id)?;
        let this =
            Arc::new(Self { raft: PandaRaft::new(node_id, Arc::new(config), network, storage) });
        Arc::clone(&this).spawn_pg_server(pg_addr).await?;
        Ok(this)
    }

    fn new_session(self: Arc<Self>) -> PandaSession {
        PandaSession { engine: self }
    }

    async fn spawn_pg_server(
        self: Arc<Self>,
        pg_addr: impl ToSocketAddrs + Send + 'static,
    ) -> PandaResult<()> {
        tokio::spawn(PgServer::new(self).handle_pg_connections(pg_addr));
        Ok(())
    }
}

#[cfg(test)]
mod tests;
