use panda_db::PandaRaft;

use crate::PandaEngine;

pub struct Transaction<'a> {
    pub(crate) engine: &'a PandaEngine,
    pub(crate) raft: PandaRaft,
}

impl<'a> Transaction<'a> {
    pub fn new(engine: &'a PandaEngine) -> Self {
        Self { engine, raft: engine.raft.clone() }
    }
}
