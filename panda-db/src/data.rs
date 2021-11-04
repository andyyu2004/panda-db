use serde::{Deserialize, Serialize};
use sqlparser::ast;

#[derive(Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: ast::ObjectName,
    pub columns: Vec<ast::ColumnDef>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ResultSet {}
