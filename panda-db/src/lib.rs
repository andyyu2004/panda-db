#[macro_use]
extern crate async_trait;

#[macro_use]
extern crate serde;

#[macro_use]
extern crate tracing;

mod cmd;
mod raft;

pub mod data;
mod rpc;

pub use cmd::*;
pub use raft::*;

pub type PandaResult<T> = anyhow::Result<T>;
pub type PandaError = anyhow::Error;

pub const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:26629";
