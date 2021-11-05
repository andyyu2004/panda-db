mod storage;

use std::sync::Arc;

use async_raft::raft::*;
use async_raft::storage::{HardState, InitialState};
use async_raft::{AppData, AppDataResponse, NodeId, Raft, RaftNetwork, RaftStorage};
use futures::prelude::*;
use tarpc::server::{BaseChannel, Channel};
use thiserror::Error;
use tokio::net::ToSocketAddrs;

use crate::cmd::PandaCmd;
use crate::data::ResultSet;
use crate::rpc::{PandaRpc, PandaRpcClient};
use crate::PandaResult;

pub use storage::PandaStorage;

pub type PandaRaft = Raft<PandaCmd, PandaResponse, PandaNetwork, PandaStorage>;

impl AppData for PandaCmd {
}

type PandaResponse = ResultSet;

impl AppDataResponse for PandaResponse {
}

macro_rules! codec {
    () => {{ tarpc::tokio_serde::formats::Bincode::default }};
}

#[derive(Debug, Default)]
pub struct PandaNetwork {
    nodes: Vec<RaftNode>,
}

impl PandaNetwork {
    pub async fn bind<A: ToSocketAddrs>(
        addr: impl ToSocketAddrs,
        join_addrs: impl IntoIterator<Item = A>,
    ) -> PandaResult<Arc<Self>> {
        let listener = tarpc::serde_transport::tcp::listen(addr, codec!()).await?;

        let nodes = future::try_join_all(join_addrs.into_iter().map(RaftNode::connect)).await?;
        let this = Arc::new(Self { nodes });
        let server = Arc::clone(&this);
        tokio::spawn(async move {
            listener
                .filter_map(|r| async move { r.ok() })
                .map(BaseChannel::with_defaults)
                .map(|channel| channel.execute(Arc::clone(&server).serve()))
                .buffer_unordered(10)
                .for_each(|()| async {})
                .await;
        });
        Ok(this)
    }
}

#[derive(Debug)]
pub struct RaftNode {
    client: PandaRpcClient,
}

impl RaftNode {
    pub async fn connect(addr: impl tokio::net::ToSocketAddrs) -> PandaResult<Self> {
        let transport = tarpc::serde_transport::tcp::connect(addr, codec!()).await?;
        let config = tarpc::client::Config::default();
        let client = PandaRpcClient::new(config, transport).spawn();
        Ok(RaftNode { client })
    }

    async fn append_entries(
        &self,
        request: AppendEntriesRequest<PandaCmd>,
    ) -> PandaResult<AppendEntriesResponse> {
        todo!()
    }
}

#[async_trait]
impl RaftNetwork<PandaCmd> for PandaNetwork {
    async fn append_entries(
        &self,
        target: NodeId,
        rpc: AppendEntriesRequest<PandaCmd>,
    ) -> anyhow::Result<AppendEntriesResponse> {
        self.nodes[target as usize].append_entries(rpc).await
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
