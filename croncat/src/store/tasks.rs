use chrono::Utc;
use color_eyre::{eyre::eyre, Report};
use croncat_sdk_tasks::types::TaskInfo;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
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
    pub events: BTreeMap<u64, HashMap<String, TaskInfo>>,
}

/// Store key pairs on disk and allow access to the data.
pub struct LocalEventStorage {
    pub path: PathBuf,
    data: Option<LocalEventsStorageEntry>,
}

impl LocalEventStorage {
    /// Create a new [`LocalEventStorage`] instance with the default directory.
    pub fn new() -> Self {
        Self::from_path(get_storage_path())
    }

    /// Create a [`LocalEventStorage`] instance at a specified path,
    /// if the data already exists at the directory we load it.
    pub fn from_path(path: PathBuf) -> Self {
        let data_file = path.join(LOCAL_STORAGE_FILENAME);

        // Load from the agent data file if it exists
        if data_file.exists() {
            let json_data = fs::read_to_string(data_file).unwrap();
            let data =
                serde_json::from_str(json_data.as_str()).expect("Failed to parse agent JSON data");
            Self { path, data }
        } else {
            // Otherwise create a new hashmap
            Self { path, data: None }
        }
    }

    /// Write our data to disk at the specified location.
    pub fn write_to_disk(&self) -> Result<(), Report> {
        if self.data.is_none() {
            return Err(eyre!("No factory data to write"));
        }
        let data_file = self.path.join(LOCAL_STORAGE_FILENAME);

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
    pub fn insert(&mut self, index: u64, events: Vec<(String, TaskInfo)>) -> Result<(), Report> {
        // Expires after 1 hour, updates any time we get new data
        let dt = Utc::now();
        let expires = dt.timestamp().saturating_add(60 * 60);

        if let Some(mut data) = self.data.clone() {
            let mut event_range = data
                .events
                .get(&index)
                .unwrap_or(&HashMap::new())
                .to_owned();
            for (k, v) in events {
                event_range.insert(k, v);
            }
            data.events.insert(index, event_range);
            self.data = Some(data);
        } else {
            let mut e: BTreeMap<u64, HashMap<String, TaskInfo>> = BTreeMap::new();
            let mut items: HashMap<String, TaskInfo> = HashMap::new();
            for (k, v) in events {
                items.insert(k, v);
            }
            e.insert(index, items);
            self.data = Some(LocalEventsStorageEntry { expires, events: e });
        }

        self.write_to_disk()?;
        Ok(())
    }

    /// Clear all data, helpful for refreshing all data
    pub fn clear_all(&mut self) -> Result<(), Report> {
        // Expires immediately, so we know to grab moar datazzzz
        let dt = Utc::now();
        let expires = dt.timestamp();
        let events: BTreeMap<u64, HashMap<String, TaskInfo>> = BTreeMap::new();
        self.data = Some(LocalEventsStorageEntry { expires, events });
        self.write_to_disk()?;
        Ok(())
    }

    /// Clear all data less than or equal to an index, but NOT 0th index
    pub fn clear_lte_index(&mut self, index: &u64) -> Result<(), Report> {
        if let Some(mut data) = self.data.clone() {
            // NOTE: use of unstable library feature 'btree_drain_filter' -- see issue #70530 <https://github.com/rust-lang/rust/issues/70530> for more information
            // data.events.drain_filter(|k, _v| k <= index && k != &0).collect();
            data.events.retain(|k, _v| k > index && k != &0);
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
    pub fn get_events_by_index(&self, index: Option<u64>) -> Option<Vec<&TaskInfo>> {
        if !self.is_expired() && self.has_events() {
            if let Some(data) = self.data.as_ref() {
                let idx = index.unwrap_or_default();
                let evts = data.events.get(&idx);
                evts.map(|e| e.values().collect())
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Totals for 0th and ranged task amounts
    pub fn get_stats(&self) -> (u64, u64) {
        if let Some(data) = self.data.clone() {
            if !data.events.is_empty() {
                let base_total: u64 = if let Some(hm_tasks) = data.events.get(&0) {
                    hm_tasks.len() as u64
                } else {
                    0
                };
                let mut range_total: u64 = 0;
                for (_k, hm_tasks) in data.events.range(1..) {
                    range_total = range_total.saturating_add(hm_tasks.len() as u64);
                }
                (base_total, range_total)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
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
            !data.events.is_empty()
        } else {
            false
        }
    }
}

impl Default for LocalEventStorage {
    fn default() -> Self {
        Self::new()
    }
}
