mod storage;

use async_raft::raft::*;
use async_raft::storage::{CurrentSnapshotData, HardState, InitialState};
use async_raft::{AppData, AppDataResponse, NodeId, Raft, RaftNetwork, RaftStorage};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use storage::PandaStorage;

#[derive(Clone, Serialize, Deserialize)]
pub enum Panda {}

impl AppData for Panda {
}

#[derive(Clone, Serialize, Deserialize)]
struct PandaResponse {}

impl AppDataResponse for PandaResponse {
}

struct PandaNetwork;

#[async_trait]
impl RaftNetwork<Panda> for PandaNetwork {
    async fn append_entries(
        &self,
        target: NodeId,
        rpc: AppendEntriesRequest<Panda>,
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

type PandaRaft = Raft<Panda, PandaResponse, PandaNetwork, PandaStorage>;
