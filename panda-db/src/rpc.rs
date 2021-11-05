use std::sync::Arc;

use super::*;
use async_raft::NodeId;
use tarpc::context;

pub type PandaRpcResult<T> = Result<T, PandaRpcError>;

#[derive(Debug, Serialize, Deserialize)]
pub enum PandaRpcError {}

#[tarpc::service]
pub trait PandaRpc {
    async fn join() -> PandaRpcResult<NodeId>;
}

#[tarpc::server]
impl PandaRpc for Arc<PandaNetwork> {
    async fn join(self, _cx: context::Context) -> PandaRpcResult<NodeId> {
        todo!()
    }
}
