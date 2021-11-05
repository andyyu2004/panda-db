#[macro_use]
extern crate async_trait;

#[macro_use]
extern crate serde;

mod cmd;
mod raft;

pub mod data;
mod rpc;

pub use cmd::*;
pub use raft::*;

pub type PandaResult<T> = anyhow::Result<T>;
