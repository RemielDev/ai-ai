//! Shared in-process state: settings cache, variation-seed memory.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use sha2::{Digest, Sha256};

use crate::suggester::Suggestion;

/// LRU-ish cache mapping (response_hash, steer_hash) -> generation history.
/// Used so re-pressing the summon hotkey with unchanged context generates
/// genuinely different suggestions.
#[derive(Default)]
pub struct VariationCache {
    inner: Mutex<HashMap<String, VariationEntry>>,
    order: Mutex<Vec<String>>,
}

#[derive(Clone, Default)]
pub struct VariationEntry {
    pub seed: u32,
    pub prior_suggestions: Vec<Suggestion>,
}

const MAX_CACHE_ENTRIES: usize = 8;

impl VariationCache {
    pub fn key(response: &str, steer: &str) -> String {
        let mut h = Sha256::new();
        h.update(response.as_bytes());
        h.update(b"\0");
        h.update(steer.as_bytes());
        hex::encode(h.finalize())
    }

    pub fn next(&self, response: &str, steer: &str) -> VariationEntry {
        let key = Self::key(response, steer);
        let mut inner = self.inner.lock();
        let mut order = self.order.lock();

        let entry_clone = {
            let entry = inner
                .entry(key.clone())
                .or_insert_with(VariationEntry::default);
            entry.seed = entry.seed.wrapping_add(1);
            entry.clone()
        };

        // Update LRU order
        order.retain(|k| k != &key);
        order.push(key.clone());
        if order.len() > MAX_CACHE_ENTRIES {
            let drop = order.remove(0);
            inner.remove(&drop);
        }

        entry_clone
    }

    pub fn remember(&self, response: &str, steer: &str, suggestions: Vec<Suggestion>) {
        let key = Self::key(response, steer);
        let mut inner = self.inner.lock();
        if let Some(entry) = inner.get_mut(&key) {
            // Keep last 12 across rounds so prompt stays bounded.
            let mut combined = entry.prior_suggestions.clone();
            combined.extend(suggestions);
            if combined.len() > 12 {
                let excess = combined.len() - 12;
                combined.drain(0..excess);
            }
            entry.prior_suggestions = combined;
        }
    }
}

/// In-memory app state. Shared as Arc<AppState> via Tauri's `.manage`.
pub struct AppState {
    pub variation: Arc<VariationCache>,
    /// Current batch of suggestions visible in the overlay.
    pub current_batch: Mutex<Vec<Suggestion>>,
    /// The chat snapshot that produced `current_batch`, used for variation
    /// retries.
    pub current_snapshot: Mutex<Option<crate::reader::ChatSnapshot>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            variation: Arc::new(VariationCache::default()),
            current_batch: Mutex::new(Vec::new()),
            current_snapshot: Mutex::new(None),
        }
    }
}
