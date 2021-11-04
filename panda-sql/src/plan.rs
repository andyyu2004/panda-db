use async_raft::raft::ClientWriteRequest;
use panda_db::data::{ResultSet, Table};
use panda_db::{PandaCmd, PandaResult};
use sqlparser::ast;

use crate::transaction::Transaction;

pub struct QueryPlan {
    root: Node,
}

impl QueryPlan {
    pub fn new(root: Node) -> QueryPlan {
        QueryPlan { root }
    }

    pub fn plan(stmt: ast::Statement) -> PandaResult<QueryPlan> {
        let node = match stmt {
            ast::Statement::CreateTable { name, columns, .. } =>
                Node::CreateTable { table: Table { name, columns } },
            _ => todo!(),
        };
        Ok(QueryPlan::new(node))
    }

    /// Lower the query plan into a that can be executed on the raft
    pub fn lower(self) -> PandaResult<PandaCmd> {
        let cmd = match self.root {
            Node::CreateTable { table } => PandaCmd::CreateTable { table },
        };
        Ok(cmd)
    }

    pub async fn execute(self, txn: &Transaction<'_>) -> PandaResult<ResultSet> {
        let cmd = self.lower()?;
        let response = txn.raft.client_write(ClientWriteRequest::new(cmd)).await?;
        Ok(response.data)
    }
}

pub enum Node {
    CreateTable { table: Table },
}
