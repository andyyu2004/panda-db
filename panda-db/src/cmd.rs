use crate::data::Table;

#[derive(Clone, Serialize, Deserialize)]
pub enum PandaCmd {
    CreateTable { table: Table },
}
