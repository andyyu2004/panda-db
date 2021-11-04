use serde::{Deserialize, Serialize};

use crate::data::Table;

#[derive(Clone, Serialize, Deserialize)]
pub enum PandaCmd {
    CreateTable { table: Table },
}
