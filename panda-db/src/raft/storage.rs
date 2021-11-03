use super::*;
use crate::PandaResult;
use async_raft::storage::CurrentSnapshotData;
use rocksdb::{DBIteratorWithThreadMode, Direction, IteratorMode, WriteBatch};
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};

const HARD_STATE_KEY: &[u8] = b"hardstate";

pub type LogEntry = Entry<Panda>;

pub struct PandaStateMachine {
    last_applied_log: AtomicU64,
}

pub(crate) struct PandaStorage {
    node_id: NodeId,
    db: rocksdb::DB,
    sm: PandaStateMachine,
}

impl PandaStorage {
    pub fn iter(&self) -> Iter<'_> {
        self.into_iter()
    }

    fn get_hard_state(&self) -> PandaResult<Option<HardState>> {
        Ok(self
            .db
            .get(HARD_STATE_KEY)?
            .map(|bytes| bincode::deserialize(&bytes).expect("deserialization should not fail")))
    }

    pub(crate) fn last_entry(&self) -> Option<Entry<Panda>> {
        self.iter().next().map(|(_, v)| v)
    }

    fn range(&self, range: Range<u64>) -> RangeIter<'_> {
        let start = range.start.to_be_bytes();
        let iter = Iter { iter: self.db.iterator(IteratorMode::From(&start, Direction::Forward)) };
        RangeIter { iter, end: range.end }
    }
}

macro_rules! parse_key {
    ($key:expr) => {
        u64::from_le_bytes((*$key).try_into().unwrap())
    };
}

pub struct RangeIter<'a> {
    iter: Iter<'a>,
    end: u64,
}

impl Iterator for RangeIter<'_> {
    type Item = (u64, Entry<Panda>);

    fn next(&mut self) -> Option<Self::Item> {
        let (k, v) = self.iter.next()?;
        if k >= self.end {
            return None;
        }
        Some((k, v))
    }
}

/// An iterator wrapping the RocksDB iterator that deserializes the bytes appropriately
pub struct Iter<'a> {
    iter: DBIteratorWithThreadMode<'a, rocksdb::DB>,
}

impl Iterator for Iter<'_> {
    type Item = (u64, Entry<Panda>);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(k, v)| {
            let k = parse_key!(k);
            let v = bincode::deserialize(&v).unwrap();
            (k, v)
        })
    }
}

impl<'a> IntoIterator for &'a PandaStorage {
    type IntoIter = Iter<'a>;
    type Item = <Self::IntoIter as IntoIterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        Iter { iter: self.db.iterator(IteratorMode::End) }
    }
}

#[derive(Debug, Error)]
pub enum ShutdownError {}

#[async_trait]
impl RaftStorage<Panda, PandaResponse> for PandaStorage {
    type ShutdownError = ShutdownError;
    type Snapshot = tokio::fs::File;

    async fn get_membership_config(&self) -> anyhow::Result<MembershipConfig> {
        let membership = self
            .iter()
            .find_map(|(_, entry)| match &entry.payload {
                EntryPayload::ConfigChange(cfg) => Some(cfg.membership.clone()),
                EntryPayload::SnapshotPointer(snapshot) => Some(snapshot.membership.clone()),
                _ => None,
            })
            .unwrap_or_else(|| MembershipConfig::new_initial(self.node_id));
        Ok(membership)
    }

    async fn get_initial_state(&self) -> anyhow::Result<InitialState> {
        let membership = self.get_membership_config().await?;
        match self.get_hard_state()? {
            Some(hard_state) => {
                let (last_log_index, last_log_term) =
                    self.last_entry().map(|entry| (entry.index, entry.term)).unwrap_or((0, 0));
                let last_applied_log = self.sm.last_applied_log.load(Ordering::SeqCst);
                Ok(InitialState {
                    last_log_index,
                    last_log_term,
                    hard_state,
                    membership,
                    last_applied_log,
                })
            }
            None => {
                let state = InitialState::new_initial(self.node_id);
                self.save_hard_state(&state.hard_state).await?;
                Ok(state)
            }
        }
    }

    async fn save_hard_state(&self, hs: &HardState) -> anyhow::Result<()> {
        Ok(self.db.put(HARD_STATE_KEY, bincode::serialize(hs).unwrap())?)
    }

    async fn get_log_entries(&self, start: u64, stop: u64) -> anyhow::Result<Vec<Entry<Panda>>> {
        Ok(self.range(start..stop).map(|(_, v)| v).collect())
    }

    async fn delete_logs_from(&self, start: u64, stop: Option<u64>) -> anyhow::Result<()> {
        let mut batch = WriteBatch::default();
        let stop = stop.unwrap_or(u64::MAX);
        batch.delete_range(start.to_be_bytes(), stop.to_be_bytes());
        self.db.write(batch)?;
        Ok(())
    }

    async fn append_entry_to_log(&self, entry: &Entry<Panda>) -> anyhow::Result<()> {
        Ok(self.db.put(entry.index.to_be_bytes(), bincode::serialize(&entry).unwrap())?)
    }

    async fn replicate_to_log(&self, entries: &[Entry<Panda>]) -> anyhow::Result<()> {
        let mut batch = WriteBatch::default();
        for entry in entries {
            batch.put(entry.index.to_be_bytes(), bincode::serialize(&entry).unwrap());
        }
        self.db.write(batch)?;
        Ok(())
    }

    async fn apply_entry_to_state_machine(
        &self,
        index: &u64,
        data: &Panda,
    ) -> anyhow::Result<PandaResponse> {
        todo!()
    }

    async fn replicate_to_state_machine(&self, entries: &[(&u64, &Panda)]) -> anyhow::Result<()> {
        todo!()
    }

    async fn do_log_compaction(&self) -> anyhow::Result<CurrentSnapshotData<Self::Snapshot>> {
        todo!()
    }

    async fn create_snapshot(&self) -> anyhow::Result<(String, Box<Self::Snapshot>)> {
        todo!()
    }

    async fn finalize_snapshot_installation(
        &self,
        index: u64,
        term: u64,
        delete_through: Option<u64>,
        id: String,
        snapshot: Box<Self::Snapshot>,
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn get_current_snapshot(
        &self,
    ) -> anyhow::Result<Option<CurrentSnapshotData<Self::Snapshot>>> {
        todo!()
    }
}
