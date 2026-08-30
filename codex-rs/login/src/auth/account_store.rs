use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;

use super::AuthDotJson;

const ACCOUNT_PROFILES_VERSION: u32 = 1;
const MAX_PROFILE_LABEL_CHARS: usize = 80;

/// Stable, non-secret identity and display metadata for an account profile.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AccountProfileMetadata {
    pub id: String,
    pub label: String,
}

/// A stored account that is not currently active.
#[derive(Clone, Deserialize, PartialEq, Serialize)]
pub struct InactiveAccountProfile {
    #[serde(flatten)]
    pub metadata: AccountProfileMetadata,
    pub auth: AuthDotJson,
}

/// Versioned metadata and inactive credentials attached to the legacy auth payload.
#[derive(Clone, Deserialize, PartialEq, Serialize)]
pub struct AccountProfiles {
    pub version: u32,
    pub active_profile: AccountProfileMetadata,
    pub inactive_profiles: Vec<InactiveAccountProfile>,
}

impl AccountProfiles {
    pub(crate) fn validate(&self) -> std::io::Result<()> {
        if self.version != ACCOUNT_PROFILES_VERSION {
            return Err(invalid_data(format!(
                "unsupported account profiles version {}",
                self.version
            )));
        }
        validate_metadata(&self.active_profile)?;

        let mut profile_ids = HashSet::from([self.active_profile.id.as_str()]);
        for profile in &self.inactive_profiles {
            validate_metadata(&profile.metadata)?;
            if !profile_ids.insert(profile.metadata.id.as_str()) {
                return Err(invalid_data("duplicate account profile id"));
            }
        }
        Ok(())
    }
}

/// Auth document with active credentials at the legacy root for compatibility.
#[derive(Clone, Deserialize, PartialEq, Serialize)]
pub struct AuthAccountStore {
    #[serde(flatten)]
    pub active_auth: AuthDotJson,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_profiles: Option<AccountProfiles>,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) pending_keyring_write: bool,
}

impl std::fmt::Debug for AuthAccountStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AuthAccountStore { <redacted> }")
    }
}

impl AuthAccountStore {
    pub fn legacy(active_auth: AuthDotJson) -> Self {
        Self {
            active_auth,
            account_profiles: None,
            pending_keyring_write: false,
        }
    }

    pub(crate) fn validate(&self) -> std::io::Result<()> {
        if let Some(profiles) = &self.account_profiles {
            profiles.validate()?;
        }
        Ok(())
    }
}

fn validate_metadata(metadata: &AccountProfileMetadata) -> std::io::Result<()> {
    if metadata.id.trim().is_empty() {
        return Err(invalid_data("account profile id must not be empty"));
    }
    let label = metadata.label.trim();
    if label.is_empty() {
        return Err(invalid_data("account profile label must not be empty"));
    }
    if label.chars().count() > MAX_PROFILE_LABEL_CHARS {
        return Err(invalid_data(format!(
            "account profile label must not exceed {MAX_PROFILE_LABEL_CHARS} characters"
        )));
    }
    Ok(())
}

fn invalid_data(message: impl Into<String>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message.into())
}
