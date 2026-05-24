//! License gate. v1 ships in dev-bypass mode that always returns Valid.
//!
//! When a license server is deployed, swap `validate_with_server` into
//! `status()`. The dev-bypass exists so the app is usable today while
//! the Lemon Squeezy + Cloudflare Worker integration is set up.

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
        tier: "dev-bypass".into(),
        message: "License gate disabled in this build.".into(),
    }
}

pub fn activate(_key: &str) -> LicenseStatus {
    // TODO: POST to https://license.ai-ai.app/v1/check when worker is deployed.
    LicenseStatus {
        valid: true,
        tier: "dev-bypass".into(),
        message: "License gate disabled in this build.".into(),
    }
}
