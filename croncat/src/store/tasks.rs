use chrono::Utc;
use color_eyre::{eyre::eyre, Report};
use cosmwasm_std::{Timestamp, Uint64};
use croncat_sdk_tasks::types::{BoundaryHeight, BoundaryTime, TaskInfo};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    ops::Bound::{Excluded, Included},
    path::PathBuf,
};

use super::get_storage_path;

/// Where our [`LocalEventStorage`] will be stored.
const LOCAL_STORAGE_FILENAME: &str = "events.json";

//
#[derive(Serialize, Deserialize, Clone)]
pub struct LocalEventsStorageEntry {
    pub expires: i64,
    // is sorted for ranged execution
    pub height_based: BTreeMap<u64, HashMap<String, TaskInfo>>,
    pub time_based: BTreeMap<u64, HashMap<String, TaskInfo>>,
}

impl std::fmt::Debug for LocalEventsStorageEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalCacheStorageEntry")
            .field("expires", &self.expires.to_string())
            .field("height_tasks", &self.height_based.keys())
            .field("time_tasks", &self.time_based.keys())
            .finish()
    }
}

pub enum EventType {
    Block,
    Time,
}

/// Store key pairs on disk and allow access to the data.
pub struct LocalEventStorage {
    pub path: PathBuf,
    pub path_prefix: Option<String>,
    data: Option<LocalEventsStorageEntry>,
}

impl LocalEventStorage {
    /// Create a new [`LocalEventStorage`] instance with the default directory.
    pub fn new(path_prefix: Option<String>) -> Self {
        let p = get_storage_path();
        Self {
            path: p,
            path_prefix,
            data: None,
        }
    }

    /// Create a [`LocalEventStorage`] instance at a specified path,
    /// if the data already exists at the directory we load it.
    pub fn from_path(&self, path: PathBuf) -> Self {
        let data_file = path
            .join(self.path_prefix.clone().unwrap_or_default())
            .join(LOCAL_STORAGE_FILENAME);

        // Load from the agent data file if it exists
        if data_file.exists() {
            let json_data = fs::read_to_string(data_file).unwrap();
            let data =
                serde_json::from_str(json_data.as_str()).expect("Failed to parse agent JSON data");
            Self {
                path,
                path_prefix: self.path_prefix.clone(),
                data,
            }
        } else {
            // Otherwise create a new hashmap
            Self {
                path,
                path_prefix: self.path_prefix.clone(),
                data: None,
            }
        }
    }

    /// Write our data to disk at the specified location.
    pub fn write_to_disk(&self) -> Result<(), Report> {
        if self.data.is_none() {
            return Err(eyre!("No factory data to write"));
        }
        let data_file = self
            .path
            .join(self.path_prefix.clone().unwrap_or_default())
            .join(LOCAL_STORAGE_FILENAME);

        // Create the directory to store our data if it doesn't exist
        if let Some(p) = data_file.parent() {
            fs::create_dir_all(p)?
        };

        let r = fs::write(
            data_file,
            serde_json::to_string_pretty(&self.data.clone().unwrap())?,
        );

        if r.is_ok() {
            Ok(())
        } else {
            Err(eyre!(r.unwrap_err()))
        }
    }

    /// Insert a items into the data set.
    pub fn update_expiry(&mut self) -> Result<(), Report> {
        // Expires after 1 hour, updates any time we get new data
        let dt = Utc::now();
        let expires = dt.timestamp().saturating_add(60); // 1 min
        let prev = self.data.as_ref().unwrap();

        self.data = Some(LocalEventsStorageEntry {
            expires,
            height_based: prev.height_based.clone(),
            time_based: prev.time_based.clone(),
        });

        self.clear_empty_indexes()?;
        self.write_to_disk()?;
        Ok(())
    }

    /// Insert a items into the data set.
    pub fn insert(
        &mut self,
        kind: EventType,
        index: u64,
        events: Vec<(String, TaskInfo)>,
    ) -> Result<(), Report> {
        // Expires after 1 hour, updates any time we get new data
        let dt = Utc::now();
        let expires = dt.timestamp().saturating_add(60); // 1 min

        if let Some(mut data) = self.data.clone() {
            match kind {
                EventType::Block => {
                    let mut event_range = data
                        .height_based
                        .get(&index)
                        .unwrap_or(&HashMap::new())
                        .to_owned();
                    for (k, v) in events {
                        event_range.insert(k, v);
                    }
                    data.height_based.insert(index, event_range);
                }
                EventType::Time => {
                    let mut event_range = data
                        .time_based
                        .get(&index)
                        .unwrap_or(&HashMap::new())
                        .to_owned();
                    for (k, v) in events {
                        event_range.insert(k, v);
                    }
                    data.time_based.insert(index, event_range);
                }
            }
            self.data = Some(data);
        } else {
            let mut height_based: BTreeMap<u64, HashMap<String, TaskInfo>> = BTreeMap::new();
            let mut time_based: BTreeMap<u64, HashMap<String, TaskInfo>> = BTreeMap::new();
            let mut items: HashMap<String, TaskInfo> = HashMap::new();
            for (k, v) in events {
                items.insert(k, v);
            }

            match kind {
                EventType::Block => {
                    height_based.insert(index, items);
                }
                EventType::Time => {
                    time_based.insert(index, items);
                }
            }
            self.data = Some(LocalEventsStorageEntry {
                expires,
                height_based,
                time_based,
            });
        }

        self.clear_empty_indexes()?;
        self.write_to_disk()?;
        Ok(())
    }

    /// Clear all data, helpful for refreshing all data
    pub fn clear_all(&mut self) -> Result<(), Report> {
        // Expires immediately, so we know to grab moar datazzzz
        let dt = Utc::now();
        let expires = dt.timestamp();
        let height_based: BTreeMap<u64, HashMap<String, TaskInfo>> = BTreeMap::new();
        let time_based: BTreeMap<u64, HashMap<String, TaskInfo>> = BTreeMap::new();
        self.data = Some(LocalEventsStorageEntry {
            expires,
            height_based,
            time_based,
        });
        self.write_to_disk()?;
        Ok(())
    }

    /// Cleaning up empty indexs
    pub fn clear_empty_indexes(&mut self) -> Result<(), Report> {
        if let Some(mut data) = self.data.clone() {
            if !data.height_based.is_empty() {
                data.height_based.retain(|_, v| !v.is_empty());
            }
            if !data.time_based.is_empty() {
                data.time_based.retain(|_, v| !v.is_empty());
            }
            self.data = Some(data);
        }

        self.write_to_disk()?;
        Ok(())
    }

    /// Remove ended tasks from cache, return the vec of task hashes for agent to
    /// check if those tasks still exist on chain and need to get removed
    pub fn clear_ended_tasks(
        &mut self,
        block_height: &u64,
        block_time: &Timestamp,
    ) -> Result<Vec<String>, Report> {
        let mut cleared = Vec::new();

        if let Some(data) = self.data.as_mut() {
            for (_, indexed_set) in data.height_based.iter_mut() {
                let mut to_remove = Vec::new();

                for (hash, task) in indexed_set.iter() {
                    // Check task end boundary, if any
                    if let croncat_sdk_tasks::types::Boundary::Height(BoundaryHeight {
                        end: Some(end),
                        ..
                    }) = task.boundary
                    {
                        // compare against current height
                        if end < Uint64::from(*block_height) {
                            to_remove.push(hash.clone());
                            cleared.push(task.task_hash.clone());
                        }
                    }
                }

                for hash in to_remove {
                    indexed_set.remove(&hash);
                }
            }
            for (_, indexed_set) in data.time_based.iter_mut() {
                let mut to_remove = Vec::new();

                for (hash, task) in indexed_set.iter() {
                    // Check task end boundary, if any
                    if let croncat_sdk_tasks::types::Boundary::Time(BoundaryTime {
                        end: Some(end),
                        ..
                    }) = task.boundary
                    {
                        // compare against current block time
                        if end < *block_time {
                            to_remove.push(hash.clone());
                            cleared.push(task.task_hash.clone());
                        }
                    }
                }

                for hash in to_remove {
                    indexed_set.remove(&hash);
                }
            }

            data.time_based.retain(|_, v| !v.is_empty());

            self.write_to_disk()?;
        }

        Ok(cleared)
    }

    /// Remove a task by task hash, dont care if wasn't there
    pub fn remove_task_by_hash(&mut self, task_hash: &str) -> Result<(), Report> {
        if let Some(data) = self.data.as_mut() {
            for (_, indexed_set) in data.height_based.iter_mut() {
                if indexed_set.contains_key(task_hash) {
                    indexed_set.remove(task_hash);
                }
            }
            for (_, indexed_set) in data.time_based.iter_mut() {
                if indexed_set.contains_key(task_hash) {
                    indexed_set.remove(task_hash);
                }
            }
        }

        Ok(())
    }

    /// Clear all data less than or equal to an index, but NOT 0th index
    /// TODO: Consider cleaning up empty indexs
    pub fn clear_lte_index(&mut self, index: &u64, kind: EventType) -> Result<(), Report> {
        if let Some(mut data) = self.data.clone() {
            // NOTE: use of unstable library feature 'btree_drain_filter' -- see issue #70530 <https://github.com/rust-lang/rust/issues/70530> for more information
            // data.events.drain_filter(|k, _v| k <= index && k != &0).collect();
            match kind {
                EventType::Block => {
                    data.height_based.retain(|k, _v| k > index && k != &0);
                }
                EventType::Time => {
                    data.time_based.retain(|k, _v| k > index && k != &0);
                }
            }
            self.data = Some(data);
        }

        self.write_to_disk()?;
        Ok(())
    }

    /// Retrieve data, only if not expired
    pub fn get(&self) -> Option<&LocalEventsStorageEntry> {
        if !self.is_expired() && self.has_events() {
            self.data.as_ref()
        } else {
            None
        }
    }

    /// Retrieve ranged events
    /// NOTE: non-ranged tasks store at index 0
    pub fn get_events_by_index(
        &self,
        index: Option<u64>,
        kind: EventType,
    ) -> Option<Vec<&TaskInfo>> {
        if !self.is_expired() && self.has_events() {
            if let Some(data) = self.data.as_ref() {
                let idx = index.unwrap_or_default();

                match kind {
                    EventType::Block => {
                        let evts = data.height_based.get(&idx);
                        evts.map(|e| e.values().collect())
                    }
                    EventType::Time => {
                        let evts = data.time_based.get(&idx);
                        evts.map(|e| e.values().collect())
                    }
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Retrieve expired ranged events
    pub fn get_events_lte_index(
        &self,
        index: Option<u64>,
        kind: EventType,
    ) -> Option<Vec<&TaskInfo>> {
        println!("get_events_lte_index !self.is_expired() && self.has_events() {:?} {:?}", !self.is_expired(), self.has_events());
        if !self.is_expired() && self.has_events() {
            if let Some(data) = self.data.as_ref() {
                let idx = index.unwrap_or(1);
                println!("get_events_lte_index idx {:?}", idx);
                let rng = match kind {
                    EventType::Block => data.height_based.range((Excluded(0), Included(idx))),
                    EventType::Time => data.time_based.range((Excluded(0), Included(idx))),
                };
                println!("get_events_lte_index rng {:?}", rng);
                Some(
                    rng.flat_map(|(_, e)| e.values())
                        .collect::<Vec<&TaskInfo>>(),
                )
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Totals for 0th and ranged task amounts
    pub fn get_stats(&self) -> (u64, u64, u64, u64) {
        if let Some(data) = self.data.clone() {
            if self.has_events() {
                let base_height_total: u64 = if let Some(hm_tasks) = data.height_based.get(&0) {
                    hm_tasks.len() as u64
                } else {
                    0
                };
                let base_time_total: u64 = if let Some(hm_tasks) = data.time_based.get(&0) {
                    hm_tasks.len() as u64
                } else {
                    0
                };
                let mut range_height_total: u64 = 0;
                for (_, hm_tasks) in data.height_based.range(1..) {
                    range_height_total = range_height_total.saturating_add(hm_tasks.len() as u64);
                }
                let mut range_time_total: u64 = 0;
                for (_, hm_tasks) in data.time_based.range(1..) {
                    range_time_total = range_time_total.saturating_add(hm_tasks.len() as u64);
                }
                (
                    base_height_total,
                    range_height_total,
                    base_time_total,
                    range_time_total,
                )
            } else {
                (0, 0, 0, 0)
            }
        } else {
            (0, 0, 0, 0)
        }
    }

    /// Check if the data has expired
    pub fn is_expired(&self) -> bool {
        if let Some(data) = self.data.clone() {
            let dt = Utc::now();
            let now = dt.timestamp();
            now > data.expires
        } else {
            true
        }
    }

    /// Check if has events data
    pub fn has_events(&self) -> bool {
        if let Some(data) = self.data.clone() {
            !data.height_based.is_empty() || !data.time_based.is_empty()
        } else {
            false
        }
    }
}

impl Default for LocalEventStorage {
    fn default() -> Self {
        Self::new(None)
    }
}
