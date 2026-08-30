use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;

use super::AuthDotJson;

const ACCOUNT_PROFILES_VERSION: u32 = 1;
const MAX_PROFILE_LABEL_CHARS: usize = 80;
const LEGACY_PROFILE_ID: &str = "legacy";
const LEGACY_PROFILE_LABEL: &str = "Account 1";

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

    pub(crate) fn save_profile(
        &mut self,
        metadata: AccountProfileMetadata,
        auth: AuthDotJson,
    ) -> std::io::Result<()> {
        validate_metadata(&metadata)?;
        let Some(profiles) = self.account_profiles.as_mut() else {
            return self.add_profile_to_legacy_store(metadata, auth);
        };
        if profiles.active_profile.id == metadata.id {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "the active account profile cannot be replaced by an add-profile login",
            ));
        }

        if let Some(profile) = profiles
            .inactive_profiles
            .iter_mut()
            .find(|profile| profile.metadata.id == metadata.id)
        {
            profile.metadata = metadata;
            profile.auth = auth;
        } else {
            profiles
                .inactive_profiles
                .push(InactiveAccountProfile { metadata, auth });
        }
        self.validate()
    }

    pub(crate) fn remove_inactive_profile(
        &mut self,
        profile_id: &str,
    ) -> std::io::Result<Option<AuthDotJson>> {
        let Some(profiles) = self.account_profiles.as_mut() else {
            return Ok(None);
        };
        if profiles.active_profile.id == profile_id {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "the active account profile must be switched before it can be removed",
            ));
        }
        let Some(index) = profiles
            .inactive_profiles
            .iter()
            .position(|profile| profile.metadata.id == profile_id)
        else {
            return Ok(None);
        };
        Ok(Some(profiles.inactive_profiles.remove(index).auth))
    }

    fn add_profile_to_legacy_store(
        &mut self,
        metadata: AccountProfileMetadata,
        auth: AuthDotJson,
    ) -> std::io::Result<()> {
        if metadata.id == LEGACY_PROFILE_ID {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("account profile id {LEGACY_PROFILE_ID:?} is reserved"),
            ));
        }
        self.account_profiles = Some(AccountProfiles {
            version: ACCOUNT_PROFILES_VERSION,
            active_profile: AccountProfileMetadata {
                id: LEGACY_PROFILE_ID.to_string(),
                label: LEGACY_PROFILE_LABEL.to_string(),
            },
            inactive_profiles: vec![InactiveAccountProfile { metadata, auth }],
        });
        self.validate()
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
