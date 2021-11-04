mod storage;

use async_raft::raft::*;
use async_raft::storage::{HardState, InitialState};
use async_raft::{AppData, AppDataResponse, NodeId, Raft, RaftNetwork, RaftStorage};
use thiserror::Error;

use crate::cmd::PandaCmd;
use crate::data::ResultSet;

pub use storage::PandaStorage;

pub type PandaRaft = Raft<PandaCmd, PandaResponse, PandaNetwork, PandaStorage>;

impl AppData for PandaCmd {
}

type PandaResponse = ResultSet;

impl AppDataResponse for PandaResponse {
}

pub struct PandaNetwork;

#[async_trait]
impl RaftNetwork<PandaCmd> for PandaNetwork {
    async fn append_entries(
        &self,
        target: NodeId,
        rpc: AppendEntriesRequest<PandaCmd>,
    ) -> anyhow::Result<AppendEntriesResponse> {
        todo!()
    }

    async fn install_snapshot(
        &self,
        target: NodeId,
        rpc: InstallSnapshotRequest,
    ) -> anyhow::Result<InstallSnapshotResponse> {
        todo!()
    }

    async fn vote(&self, target: NodeId, rpc: VoteRequest) -> anyhow::Result<VoteResponse> {
        todo!()
    }
}
