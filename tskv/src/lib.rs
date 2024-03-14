#![allow(dead_code)]
#![allow(unreachable_patterns)]
#![recursion_limit = "256"]

use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
pub use compaction::check::vnode_table_checksum_schema;
use compaction::{CompactTask, FlushReq};
use context::GlobalContext;
use datafusion::arrow::record_batch::RecordBatch;
use file_system::file_info::FileInfo;
use models::meta_data::{NodeId, VnodeId};
use models::predicate::domain::ColumnDomains;
use models::{SeriesId, SeriesKey};
use serde::{Deserialize, Serialize};
use summary::SummaryTask;
use tokio::sync::mpsc::Sender;
use tokio::sync::RwLock;
use version_set::VersionSet;
use vnode_store::VnodeStorage;

pub use crate::error::{Error, Result};
pub use crate::kv_option::Options;
use crate::kv_option::StorageOptions;
pub use crate::kvcore::TsKv;
pub use crate::summary::{print_summary_statistics, Summary, VersionEdit};
use crate::tseries_family::SuperVersion;
// todo: add a method for print tsm statistics
// pub use crate::tsm::print_tsm_statistics;
pub use crate::wal::print_wal_statistics;

pub mod byte_utils;
mod compaction;
mod compute;
mod context;
pub mod database;
pub mod engine_mock;
pub mod error;
pub mod file_system;
pub mod file_utils;
pub mod index;
pub mod kv_option;
mod kvcore;
mod memcache;
// TODO supposedly private
pub mod reader;
mod record_file;
mod schema;
mod summary;
mod tseries_family;
pub mod tsm;
mod version_set;
pub mod vnode_store;
pub mod wal;

/// The column file ID is unique in a KV instance
/// and uniquely corresponds to one column file.
pub type ColumnFileId = u64;
type TseriesFamilyId = VnodeId;
type LevelId = u32;

pub type EngineRef = Arc<dyn Engine>;

#[derive(PartialEq, Eq, Hash)]
pub struct UpdateSetValue<K, V> {
    pub key: K,
    pub value: Option<V>,
}

#[async_trait]
pub trait Engine: Send + Sync + Debug {
    /// open a tsfamily, if already exist just return.
    async fn open_tsfamily(
        &self,
        tenant: &str,
        db_name: &str,
        vnode_id: VnodeId,
    ) -> Result<VnodeStorage>;

    /// Remove the storage unit(caches and files) managed by engine,
    /// then remove directory of the storage unit.
    async fn remove_tsfamily(&self, tenant: &str, database: &str, vnode_id: VnodeId) -> Result<()>;

    /// Flush all caches of the storage unit into a file.
    async fn flush_tsfamily(&self, tenant: &str, database: &str, vnode_id: VnodeId) -> Result<()>;

    /// Read index of a storage unit, find series ids that matches the filter.
    async fn get_series_id_by_filter(
        &self,
        tenant: &str,
        database: &str,
        table: &str,
        vnode_id: VnodeId,
        filter: &ColumnDomains<String>,
    ) -> Result<Vec<SeriesId>>;

    /// Read index of a storage unit, get `SeriesKey` of the geiven series id.
    async fn get_series_key(
        &self,
        tenant: &str,
        database: &str,
        table: &str,
        vnode_id: VnodeId,
        series_id: &[SeriesId],
    ) -> Result<Vec<SeriesKey>>;

    /// Get a `SuperVersion` that contains the latest version of caches and files
    /// of the storage unit.
    async fn get_db_version(
        &self,
        tenant: &str,
        database: &str,
        vnode_id: u32,
    ) -> Result<Option<Arc<SuperVersion>>>;

    /// Get the storage options which was used to install the engine.
    fn get_storage_options(&self) -> Arc<StorageOptions>;

    /// For the specified storage units, flush all caches into files, then compact
    /// files into larger files.
    async fn compact(&self, vnode_ids: Vec<TseriesFamilyId>) -> Result<()>;

    /// Get a compressed hash_tree(ID and checksum of each vnode) of engine.
    async fn get_vnode_hash_tree(&self, vnode_id: VnodeId) -> Result<RecordBatch>;

    /// Close all background jobs of engine.
    async fn close(&self);
}

#[derive(Debug, Clone)]
pub struct TsKvContext {
    pub options: Arc<Options>,
    pub global_ctx: Arc<GlobalContext>,
    pub version_set: Arc<RwLock<VersionSet>>,

    pub flush_task_sender: Sender<FlushReq>,
    pub compact_task_sender: Sender<CompactTask>,
    pub summary_task_sender: Sender<SummaryTask>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VnodeSnapshot {
    pub snapshot_id: String,
    pub node_id: NodeId,
    pub vnode_id: VnodeId,
    pub last_seq_no: u64,
    pub files_info: Vec<FileInfo>,
    pub version_edit: VersionEdit,
}

pub mod test {
    pub use crate::memcache::test::{get_one_series_cache_data, put_rows_to_cache};
}
