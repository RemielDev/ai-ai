//! Shared in-process state.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Datelike, Utc};
use parking_lot::Mutex;
use sha2::{Digest, Sha256};

use std::collections::VecDeque;
use std::time::{Duration, Instant};

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

/// Rate limiter — protects against hotkey spam (cooldown) and runaway cost
/// (rolling-window cap). Both checks are cheap and lock-free in the common path.
pub struct RateLimit {
    last_summon:  Mutex<Option<Instant>>,
    last_improve: Mutex<Option<Instant>>,
    window:       Mutex<VecDeque<Instant>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RateAction { Summon, Improve, Regenerate }

#[derive(Debug)]
pub enum RateVerdict {
    Allowed,
    Cooldown { wait_ms: u128 },
    WindowExceeded { resets_in_ms: u128 },
}

impl RateLimit {
    pub fn new() -> Self {
        Self {
            last_summon:  Mutex::new(None),
            last_improve: Mutex::new(None),
            window:       Mutex::new(VecDeque::with_capacity(80)),
        }
    }

    /// Hard caps: 60 calls per 60s rolling window.
    const WINDOW_MAX: usize = 60;
    const WINDOW_DURATION: Duration = Duration::from_secs(60);

    /// Cooldowns per action to prevent accidental double-fire.
    fn cooldown(action: RateAction) -> Duration {
        match action {
            RateAction::Summon     => Duration::from_millis(800),
            RateAction::Improve    => Duration::from_millis(1200),
            RateAction::Regenerate => Duration::from_millis(600),
        }
    }

    pub fn check(&self, action: RateAction) -> RateVerdict {
        let now = Instant::now();

        // 1) Per-action cooldown
        let slot = match action {
            RateAction::Summon | RateAction::Regenerate => &self.last_summon,
            RateAction::Improve => &self.last_improve,
        };
        if let Some(last) = *slot.lock() {
            let cd = Self::cooldown(action);
            let elapsed = now.saturating_duration_since(last);
            if elapsed < cd {
                return RateVerdict::Cooldown { wait_ms: (cd - elapsed).as_millis() };
            }
        }

        // 2) Rolling window
        let mut win = self.window.lock();
        while let Some(&front) = win.front() {
            if now.saturating_duration_since(front) > Self::WINDOW_DURATION {
                win.pop_front();
            } else {
                break;
            }
        }
        if win.len() >= Self::WINDOW_MAX {
            let resets_in = if let Some(&front) = win.front() {
                Self::WINDOW_DURATION.saturating_sub(now.saturating_duration_since(front))
            } else {
                Duration::ZERO
            };
            return RateVerdict::WindowExceeded { resets_in_ms: resets_in.as_millis() };
        }

        // Commit: record fire
        *slot.lock() = Some(now);
        win.push_back(now);
        RateVerdict::Allowed
    }
}

pub struct AppState {
    pub variation: Arc<VariationCache>,
    pub current_batch: Mutex<Vec<Suggestion>>,
    pub current_snapshot: Mutex<Option<crate::reader::ChatSnapshot>>,
    pub usage: Arc<UsageStats>,
    pub rate_limit: Arc<RateLimit>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            variation: Arc::new(VariationCache::default()),
            current_batch: Mutex::new(Vec::new()),
            current_snapshot: Mutex::new(None),
            usage: Arc::new(UsageStats::default()),
            rate_limit: Arc::new(RateLimit::new()),
        }
    }
}
