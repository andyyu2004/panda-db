mod plan;
mod transaction;

use std::sync::Arc;

use futures::future;
use panda_db::data::ResultSet;
use panda_db::{PandaNetwork, PandaRaft, PandaResult, PandaStorage};
use sqlparser::ast;
use sqlparser::dialect::{Dialect, PostgreSqlDialect};
use sqlparser::parser::{Parser, ParserError};

use self::plan::QueryPlan;
use self::transaction::Transaction;

pub struct PandaSession {
    engine: PandaEngine,
}

const DIALECT: &'static dyn Dialect = &PostgreSqlDialect {};

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
    pub fn new() -> PandaResult<Self> {
        const CLUSTER_NAME: String = String::new();
        let config = async_raft::Config::build(CLUSTER_NAME).validate()?;
        // TODO properly assign node_id according to requirements
        let node_id = 0;
        let network = Arc::new(PandaNetwork);
        let storage = PandaStorage::new(node_id)?;
        Ok(Self { raft: PandaRaft::new(node_id, Arc::new(config), network, storage) })
    }
}
