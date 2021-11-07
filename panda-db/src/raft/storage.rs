use super::*;
use crate::PandaResult;
use async_raft::storage::CurrentSnapshotData;
use parking_lot::RwLock;
use rocksdb::{DBIterator, DBRawIterator, Direction, IteratorMode, WriteBatch};
use std::ops::Range;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};

const HARD_STATE_KEY: &[u8] = b"hardstate";

// store it under `/var/lib/panda` eventually
// maybe /tmp/panda will do for now
const SNAPSHOT_DIR: &str = "/tmp/panda/snapshots";

/// The panda raft state machine
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Panda {
    /// Index of the highest log entry applied to the state machine
    last_applied_log: AtomicU64,
}

impl Clone for Panda {
    fn clone(&self) -> Self {
        // Why are atomics not clone?
        Self { last_applied_log: AtomicU64::new(self.last_applied_log.load(Ordering::SeqCst)) }
    }
}

pub struct PandaStorage {
    node_id: NodeId,
    logdb: rocksdb::DB,
    metadb: rocksdb::DB,
    panda: RwLock<Panda>,
    next_snapshot_idx: AtomicU64,
    /// The id of the current active snapshot. u64::MAX if no snapshot is active.
    active_snapshot_id: AtomicU64,
}

impl PandaStorage {
    pub fn new(node_id: NodeId) -> PandaResult<Arc<Self>> {
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        Ok(Arc::new(Self {
            node_id,
            logdb: rocksdb::DB::open(&opts, "/tmp/panda/logdb")?,
            metadb: rocksdb::DB::open(&opts, "/tmp/panda/metadb")?,
            panda: Default::default(),
            next_snapshot_idx: Default::default(),
            active_snapshot_id: AtomicU64::new(u64::MAX),
        }))
    }

    pub fn iter(&self) -> Iter<'_> {
        self.into_iter()
    }

    /// Create an iterator that performs a iteration from the most recent entry to the oldest
    pub fn values(&self) -> ValueIter<'_> {
        let mut iter = self.logdb.raw_iterator();
        iter.seek_to_last();
        ValueIter { iter }
    }

    fn get_hard_state(&self) -> PandaResult<Option<HardState>> {
        Ok(self.metadb.get(HARD_STATE_KEY)?.map(|bytes| deserialize(&bytes)))
    }

    pub(crate) fn last_entry(&self) -> Option<Entry<PandaCmd>> {
        self.iter().next().map(|(_, v)| v)
    }

    fn get(&self, idx: u64) -> PandaResult<Entry<PandaCmd>> {
        let bytes = self.logdb.get_pinned(idx.to_be_bytes())?.unwrap();
        let entry = deserialize::<Entry<PandaCmd>>(&bytes);
        debug_assert_eq!(entry.index, idx);
        Ok(entry)
    }

    fn range(&self, range: Range<u64>) -> RangeIter<'_> {
        let start = range.start.to_be_bytes();
        let iter =
            Iter { iter: self.logdb.iterator(IteratorMode::From(&start, Direction::Forward)) };
        RangeIter { iter, end: range.end }
    }

    /// Find the first entry which contains membership config from the iterator
    fn find_membership_config_from(
        &self,
        mut iter: impl Iterator<Item = Entry<PandaCmd>>,
    ) -> MembershipConfig {
        iter.find_map(|entry| match &entry.payload {
            EntryPayload::ConfigChange(cfg) => Some(cfg.membership.clone()),
            EntryPayload::SnapshotPointer(snapshot) => Some(snapshot.membership.clone()),
            _ => None,
        })
        .unwrap_or_else(|| MembershipConfig::new_initial(self.node_id))
    }

    fn active_snapshot_file(&self) -> io::Result<Option<std::fs::File>> {
        let idx = self.active_snapshot_id.load(Ordering::SeqCst);
        if idx == u64::MAX {
            return Ok(None);
        }
        std::fs::File::open(format!("{}-{}", SNAPSHOT_DIR, idx)).map(Some)
    }

    fn next_snapshot_id(&self) -> String {
        let idx = self.next_snapshot_idx.fetch_add(1, Ordering::SeqCst);
        format!("{}", idx)
    }

    async fn create_snapshot_file(&self) -> io::Result<(String, File)> {
        let snapshot_id = self.next_snapshot_id();
        let file = File::create(Path::new(SNAPSHOT_DIR).join(&snapshot_id)).await?;
        Ok((snapshot_id, file))
    }
}

fn serialize<T: serde::Serialize>(t: &T) -> Vec<u8> {
    bincode::serialize(t).expect("serialization should not fail")
}

fn deserialize<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> T {
    bincode::deserialize(bytes).expect("deserialization should not fail")
}

macro_rules! parse_key {
    ($key:expr) => {
        u64::from_le_bytes((*$key).try_into().unwrap())
    };
}

pub struct ValueIter<'a> {
    iter: DBRawIterator<'a>,
}

impl<'a> Iterator for ValueIter<'a> {
    type Item = Entry<PandaCmd>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.prev();
        self.iter.value().map(deserialize)
    }
}

pub struct RangeIter<'a> {
    iter: Iter<'a>,
    end: u64,
}

impl Iterator for RangeIter<'_> {
    type Item = (u64, Entry<PandaCmd>);

    fn next(&mut self) -> Option<Self::Item> {
        let (k, v) = self.iter.next()?;
        if k >= self.end {
            return None;
        }
        Some((k, v))
    }
}

/// An iterator wrapping the RocksDB iterator that deserializes the bytes appropriately.
/// Yields highest keys first (i.e. the most recent logs)
pub struct Iter<'a> {
    iter: DBIterator<'a>,
}

impl Iterator for Iter<'_> {
    type Item = (u64, Entry<PandaCmd>);

    fn next(&mut self) -> Option<Self::Item> {
        // TODO can use `raw_iterator` to avoid the boxing cost as we only need a slice to deserialize from
        self.iter.next().map(|(k, v)| {
            let k = parse_key!(k);
            let v = deserialize(&v);
            (k, v)
        })
    }
}

impl<'a> IntoIterator for &'a PandaStorage {
    type IntoIter = Iter<'a>;
    type Item = <Self::IntoIter as IntoIterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        Iter { iter: self.logdb.iterator(IteratorMode::End) }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PandaSnapshot {
    term: u64,
    index: u64,
    membership: MembershipConfig,
    panda: Panda,
}

#[derive(Debug, Error)]
pub enum ShutdownError {}

#[async_trait]
impl RaftStorage<PandaCmd, PandaResponse> for PandaStorage {
    type ShutdownError = ShutdownError;
    type Snapshot = tokio::fs::File;

    async fn get_membership_config(&self) -> anyhow::Result<MembershipConfig> {
        Ok(self.find_membership_config_from(self.values()))
    }

    async fn get_initial_state(&self) -> anyhow::Result<InitialState> {
        let membership = self.get_membership_config().await?;
        match self.get_hard_state()? {
            Some(hard_state) => {
                let (last_log_index, last_log_term) =
                    self.last_entry().map(|entry| (entry.index, entry.term)).unwrap_or((0, 0));
                let last_applied_log = self.panda.read().last_applied_log.load(Ordering::SeqCst);
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
        Ok(self.metadb.put(HARD_STATE_KEY, serialize(hs))?)
    }

    async fn get_log_entries(&self, start: u64, stop: u64) -> anyhow::Result<Vec<Entry<PandaCmd>>> {
        Ok(self.range(start..stop).map(|(_, v)| v).collect())
    }

    async fn delete_logs_from(&self, start: u64, stop: Option<u64>) -> anyhow::Result<()> {
        let mut batch = WriteBatch::default();
        let stop = stop.unwrap_or(u64::MAX);
        batch.delete_range(start.to_be_bytes(), stop.to_be_bytes());
        self.logdb.write(batch)?;
        Ok(())
    }

    async fn append_entry_to_log(&self, entry: &Entry<PandaCmd>) -> anyhow::Result<()> {
        Ok(self.logdb.put(entry.index.to_be_bytes(), serialize(&entry))?)
    }

    async fn replicate_to_log(&self, entries: &[Entry<PandaCmd>]) -> anyhow::Result<()> {
        let mut batch = WriteBatch::default();
        for entry in entries {
            batch.put(entry.index.to_be_bytes(), serialize(&entry));
        }
        self.logdb.write(batch)?;
        Ok(())
    }

    async fn apply_entry_to_state_machine(
        &self,
        index: &u64,
        data: &PandaCmd,
    ) -> anyhow::Result<PandaResponse> {
        todo!()
    }

    async fn replicate_to_state_machine(
        &self,
        entries: &[(&u64, &PandaCmd)],
    ) -> anyhow::Result<()> {
        todo!()
    }

    async fn do_log_compaction(&self) -> anyhow::Result<CurrentSnapshotData<Self::Snapshot>> {
        let (index, panda) = {
            let panda_read = self.panda.read();
            let index = panda_read.last_applied_log.load(Ordering::SeqCst);
            let panda = panda_read.clone();
            (index, panda)
        };
        assert_eq!(
            index,
            panda.last_applied_log.load(Ordering::SeqCst),
            "maybe it's possible to the panda to be changed while cloning it (through interior mutability)?"
        );
        let term = self.get(index)?.term;
        let membership =
            self.find_membership_config_from(self.values().skip_while(|entry| entry.index > index));
        let snapshot = PandaSnapshot { index, term, membership: membership.clone(), panda };
        let (_, mut snapshot_file) = self.create_snapshot_file().await?;
        snapshot_file.write_all(&serialize(&snapshot)).await?;
        snapshot_file.flush().await?;
        Ok(CurrentSnapshotData { index, membership, term, snapshot: Box::new(snapshot_file) })
    }

    async fn create_snapshot(&self) -> anyhow::Result<(String, Box<Self::Snapshot>)> {
        let (id, snapshot_file) = self.create_snapshot_file().await?;
        Ok((id, Box::new(snapshot_file)))
    }

    async fn finalize_snapshot_installation(
        &self,
        index: u64,
        term: u64,
        delete_through: Option<u64>,
        id: String,
        mut snapshot: Box<Self::Snapshot>,
    ) -> anyhow::Result<()> {
        let membership =
            self.find_membership_config_from(self.values().skip_while(|entry| entry.index > index));
        self.active_snapshot_id
            .store(id.parse::<u64>().expect("invalid snapshot id"), Ordering::SeqCst);
        let entry: Entry<PandaCmd> = Entry::new_snapshot_pointer(index, term, id, membership);

        let mut batch = WriteBatch::default();
        let max = delete_through.unwrap_or(u64::MAX);
        batch.delete_range(u64::MIN.to_be_bytes(), max.to_be_bytes());
        self.logdb.put(index.to_be_bytes(), serialize(&entry))?;
        self.logdb.write(batch)?;

        let mut buf = vec![];
        snapshot.read_to_end(&mut buf).await?;
        let panda_snapshot = deserialize::<PandaSnapshot>(&buf);
        *self.panda.write() = panda_snapshot.panda;
        Ok(())
    }

    async fn get_current_snapshot(
        &self,
    ) -> anyhow::Result<Option<CurrentSnapshotData<Self::Snapshot>>> {
        let file = match self.active_snapshot_file()? {
            Some(file) => file,
            None => return Ok(None),
        };
        let PandaSnapshot { term, index, membership, .. } =
            bincode::deserialize_from::<_, PandaSnapshot>(&file)?;
        Ok(Some(CurrentSnapshotData {
            term,
            index,
            membership,
            snapshot: Box::new(File::from_std(file)),
        }))
    }
}
