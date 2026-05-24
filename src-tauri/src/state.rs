//! Shared in-process state.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Datelike, Utc};
use parking_lot::Mutex;
use sha2::{Digest, Sha256};

use crate::suggester::Suggestion;

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
            let entry = inner.entry(key.clone()).or_default();
            entry.seed = entry.seed.wrapping_add(1);
            entry.clone()
        };

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

/// Per-day suggestion counter. Resets at local-day boundary.
#[derive(Default)]
pub struct UsageStats {
    today_date: Mutex<Option<(i32, u32, u32)>>, // (year, month, day) UTC
    today_count: Mutex<u32>,
    lifetime: Mutex<u64>,
    accepts: Mutex<u64>,
}

impl UsageStats {
    fn current_day() -> (i32, u32, u32) {
        let now = Utc::now();
        (now.year(), now.month(), now.day())
    }
    pub fn record_round(&self, suggestions_in_round: usize) {
        let day = Self::current_day();
        let mut date = self.today_date.lock();
        let mut count = self.today_count.lock();
        if date.as_ref() != Some(&day) {
            *date = Some(day);
            *count = 0;
        }
        *count = count.saturating_add(suggestions_in_round as u32);
        *self.lifetime.lock() += suggestions_in_round as u64;
    }
    pub fn record_accept(&self) {
        *self.accepts.lock() += 1;
    }
    pub fn today(&self) -> u32 {
        let day = Self::current_day();
        let date = self.today_date.lock();
        if date.as_ref() == Some(&day) {
            *self.today_count.lock()
        } else {
            0
        }
    }
}

pub struct AppState {
    pub variation: Arc<VariationCache>,
    pub current_batch: Mutex<Vec<Suggestion>>,
    pub current_snapshot: Mutex<Option<crate::reader::ChatSnapshot>>,
    pub usage: Arc<UsageStats>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            variation: Arc::new(VariationCache::default()),
            current_batch: Mutex::new(Vec::new()),
            current_snapshot: Mutex::new(None),
            usage: Arc::new(UsageStats::default()),
        }
    }
}
