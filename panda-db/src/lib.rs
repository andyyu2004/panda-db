#[macro_use]
extern crate async_trait;

mod raft;

pub type PandaResult<T> = anyhow::Result<T>;
