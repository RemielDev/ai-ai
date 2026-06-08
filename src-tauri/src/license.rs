//! Build identity.
//!
//! This open-source build is permanently free under MIT. The license/trial
//! plumbing exists so the future Pro fork can swap in a real check without
//! changing the UI contract.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub valid: bool,
    pub tier: String,
    pub message: String,
}

pub fn status() -> LicenseStatus {
    LicenseStatus {
        valid: true,
        tier: "Free · MIT".into(),
        message: "Open-source build. Every feature unlocked, forever.".into(),
    }
}

pub fn activate(_key: &str) -> LicenseStatus {
    // No license keys in the OS build. Pro version (separate codebase) will
    // implement real activation.
    LicenseStatus {
        valid: true,
        tier: "Free · MIT".into(),
        message: "This is the open-source build - no license needed.".into(),
    }
}
