use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;
use sha2::Sha256;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tempfile::NamedTempFile;
use tracing::warn;

use self::atomic_file::replace_auth_file;
use self::serialized::SerializedAuthStorage;
use super::AccountProfileMetadata;
use super::AuthAccountStore;
use super::BedrockAccessKeysAuth;
use super::BedrockApiKeyAuth;
use crate::token_data::TokenData;
use codex_agent_identity::AgentIdentityJwtClaims;
use codex_agent_identity::decode_agent_identity_jwt;
use codex_config::types::AuthCredentialsStoreMode;
pub use codex_config::types::AuthKeyringBackendKind;
use codex_keyring_store::DefaultKeyringStore;
use codex_keyring_store::KeyringStore;
use codex_protocol::account::PlanType as AccountPlanType;
use codex_protocol::auth::AuthMode;
use codex_secrets::LocalSecretsNamespace;
use codex_secrets::SecretName;
use codex_secrets::SecretScope;
use codex_secrets::SecretsBackendKind;
use codex_secrets::SecretsManager;
use once_cell::sync::Lazy;

mod atomic_file;
mod serialized;

/// Expected structure for $CODEX_HOME/auth.json.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct AuthDotJson {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_mode: Option<AuthMode>,

    #[serde(rename = "OPENAI_API_KEY")]
    pub openai_api_key: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<TokenData>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_refresh: Option<DateTime<Utc>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_identity: Option<AgentIdentityStorage>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personal_access_token: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bedrock_api_key: Option<BedrockApiKeyAuth>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bedrock_access_keys: Option<BedrockAccessKeysAuth>,
}

fn deserialize_auth_store(serialized: &str) -> std::io::Result<AuthAccountStore> {
    let store: AuthAccountStore = serde_json::from_str(serialized)?;
    store.validate()?;
    Ok(store)
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum AgentIdentityStorage {
    Jwt(String),
    Record(AgentIdentityAuthRecord),
}

impl AgentIdentityStorage {
    pub fn has_auth_material(&self) -> bool {
        match self {
            Self::Jwt(jwt) => !jwt.trim().is_empty(),
            Self::Record(record) => {
                !record.agent_runtime_id.trim().is_empty()
                    && !record.agent_private_key.trim().is_empty()
            }
        }
    }

    pub(crate) fn as_record(&self) -> Option<&AgentIdentityAuthRecord> {
        match self {
            Self::Jwt(_) => None,
            Self::Record(record) => Some(record),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct AgentIdentityAuthRecord {
    pub agent_runtime_id: String,
    pub agent_private_key: String,
    pub account_id: String,
    pub chatgpt_user_id: String,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_non_empty_string",
        serialize_with = "serialize_optional_string_as_empty"
    )]
    pub email: Option<String>,
    pub plan_type: AccountPlanType,
    pub chatgpt_account_is_fedramp: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

fn deserialize_optional_non_empty_string<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(|value| value.filter(|value| !value.is_empty()))
}

fn serialize_optional_string_as_empty<S>(
    value: &Option<String>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    value.as_deref().unwrap_or_default().serialize(serializer)
}

impl AgentIdentityAuthRecord {
    pub(crate) fn from_agent_identity_jwt(jwt: &str) -> std::io::Result<Self> {
        let claims =
            decode_agent_identity_jwt(jwt, /*jwks*/ None).map_err(std::io::Error::other)?;

        Ok(claims.into())
    }
}

impl From<AgentIdentityJwtClaims> for AgentIdentityAuthRecord {
    fn from(claims: AgentIdentityJwtClaims) -> Self {
        Self {
            agent_runtime_id: claims.agent_runtime_id,
            agent_private_key: claims.agent_private_key,
            account_id: claims.account_id,
            chatgpt_user_id: claims.chatgpt_user_id,
            email: claims.email,
            plan_type: claims.plan_type.into(),
            chatgpt_account_is_fedramp: claims.chatgpt_account_is_fedramp,
            task_id: None,
        }
    }
}

pub(super) fn get_auth_file(codex_home: &Path) -> PathBuf {
    codex_home.join("auth.json")
}

pub(super) fn delete_file_if_exists(codex_home: &Path) -> std::io::Result<bool> {
    delete_path_if_exists(&get_auth_file(codex_home))
}

fn delete_path_if_exists(path: &Path) -> std::io::Result<bool> {
    match std::fs::remove_file(path) {
        Ok(()) => {
            #[cfg(unix)]
            if let Some(parent) = path.parent() {
                File::open(parent)?.sync_all()?;
            }
            Ok(true)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}

pub(super) trait AuthStorageBackend: Debug + Send + Sync {
    fn load_store(&self) -> std::io::Result<Option<AuthAccountStore>>;
    fn save_store(&self, store: &AuthAccountStore) -> std::io::Result<()>;
    fn delete(&self) -> std::io::Result<bool>;

    fn load(&self) -> std::io::Result<Option<AuthDotJson>> {
        Ok(self.load_store()?.map(|store| store.active_auth))
    }

    fn save(&self, auth: &AuthDotJson) -> std::io::Result<()> {
        self.save_store(&AuthAccountStore::legacy(auth.clone()))
    }

    #[cfg(test)]
    fn update_active(&self, auth: &AuthDotJson) -> std::io::Result<()> {
        self.mutate_active(ActiveAuthMutation::Replace(Box::new(auth.clone())))
            .map(|_| ())
    }

    fn mutate_active(&self, mutation: ActiveAuthMutation) -> std::io::Result<AuthDotJson> {
        let mut store = self
            .load_store()?
            .or_else(|| mutation.seed().map(AuthAccountStore::legacy))
            .ok_or_else(|| std::io::Error::other("auth data is not available"))?;
        mutation.apply(&mut store.active_auth)?;
        let active_auth = store.active_auth.clone();
        self.save_store(&store)?;
        Ok(active_auth)
    }

    fn save_profile(
        &self,
        metadata: AccountProfileMetadata,
        auth: AuthDotJson,
    ) -> std::io::Result<()> {
        let mut store = self
            .load_store()?
            .ok_or_else(|| std::io::Error::other("active auth data is not available"))?;
        store.save_profile(metadata, auth)?;
        self.save_store(&store)
    }

    fn remove_inactive_profile(&self, profile_id: &str) -> std::io::Result<Option<AuthDotJson>> {
        let mut store = self
            .load_store()?
            .ok_or_else(|| std::io::Error::other("active auth data is not available"))?;
        let removed = store.remove_inactive_profile(profile_id)?;
        if removed.is_some() {
            self.save_store(&store)?;
        }
        Ok(removed)
    }
}

pub(super) enum ActiveAuthMutation {
    #[cfg(test)]
    Replace(Box<AuthDotJson>),
    AgentIdentity(AgentIdentityAuthRecord),
    Tokens {
        id_token: Option<crate::token_data::IdTokenInfo>,
        access_token: Option<String>,
        refresh_token: Option<String>,
        refreshed_at: DateTime<Utc>,
    },
}

impl ActiveAuthMutation {
    fn seed(&self) -> Option<AuthDotJson> {
        match self {
            #[cfg(test)]
            Self::Replace(auth) => Some(*auth.clone()),
            Self::AgentIdentity(_) | Self::Tokens { .. } => None,
        }
    }

    fn apply(self, auth: &mut AuthDotJson) -> std::io::Result<()> {
        match self {
            #[cfg(test)]
            Self::Replace(replacement) => *auth = *replacement,
            Self::AgentIdentity(record) => {
                auth.agent_identity = Some(AgentIdentityStorage::Record(record));
            }
            Self::Tokens {
                id_token,
                access_token,
                refresh_token,
                refreshed_at,
            } => {
                let tokens = auth.tokens.get_or_insert_with(TokenData::default);
                if let Some(id_token) = id_token {
                    tokens.id_token = id_token;
                }
                if let Some(access_token) = access_token {
                    tokens.access_token = access_token;
                }
                if let Some(refresh_token) = refresh_token {
                    tokens.refresh_token = refresh_token;
                }
                auth.last_refresh = Some(refreshed_at);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(super) struct FileAuthStorage {
    codex_home: PathBuf,
}

impl FileAuthStorage {
    pub(super) fn new(codex_home: PathBuf) -> Self {
        Self { codex_home }
    }

    /// Attempt to read and parse the `auth.json` file in the given `CODEX_HOME` directory.
    /// Returns the full AuthDotJson structure.
    #[cfg(test)]
    pub(super) fn try_read_auth_json(&self, auth_file: &Path) -> std::io::Result<AuthDotJson> {
        Ok(self.try_read_auth_store(auth_file)?.active_auth)
    }

    fn try_read_auth_store(&self, auth_file: &Path) -> std::io::Result<AuthAccountStore> {
        let contents = Self::read_auth_file(auth_file)?;
        deserialize_auth_store(&contents)
    }

    fn read_auth_file(auth_file: &Path) -> std::io::Result<String> {
        let mut file = File::open(auth_file)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }
}

impl AuthStorageBackend for FileAuthStorage {
    fn load_store(&self) -> std::io::Result<Option<AuthAccountStore>> {
        let auth_file = get_auth_file(&self.codex_home);
        let store = match self.try_read_auth_store(&auth_file) {
            Ok(auth) => auth,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err),
        };
        Ok(Some(store))
    }

    fn load(&self) -> std::io::Result<Option<AuthDotJson>> {
        let auth_file = get_auth_file(&self.codex_home);
        match Self::read_auth_file(&auth_file) {
            Ok(contents) => serde_json::from_str(&contents)
                .map(Some)
                .map_err(Into::into),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn save_store(&self, store: &AuthAccountStore) -> std::io::Result<()> {
        store.validate()?;
        let auth_file = get_auth_file(&self.codex_home);
        let parent = auth_file.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("auth path {} has no parent directory", auth_file.display()),
            )
        })?;
        std::fs::create_dir_all(parent)?;
        let json_data = serde_json::to_string_pretty(store)?;
        let mut temporary = NamedTempFile::new_in(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            temporary
                .as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        temporary.write_all(json_data.as_bytes())?;
        temporary.flush()?;
        temporary.as_file().sync_all()?;
        replace_auth_file(temporary, &auth_file)
    }

    fn delete(&self) -> std::io::Result<bool> {
        delete_file_if_exists(&self.codex_home)
    }
}

static CODEX_AUTH_SECRET_NAME: Lazy<SecretName> =
    Lazy::new(|| match SecretName::new("CODEX_AUTH") {
        Ok(name) => name,
        Err(err) => unreachable!("CODEX_AUTH should be a valid secret name: {err}"),
    });
const KEYRING_SERVICE: &str = "Codex Auth";

// turns codex_home path into a stable, short key string
fn canonical_storage_home(codex_home: &Path) -> PathBuf {
    codex_home.canonicalize().unwrap_or_else(|_| {
        let Some(parent) = codex_home.parent() else {
            return codex_home.to_path_buf();
        };
        let Some(file_name) = codex_home.file_name() else {
            return codex_home.to_path_buf();
        };
        parent
            .canonicalize()
            .map(|canonical_parent| canonical_parent.join(file_name))
            .unwrap_or_else(|_| codex_home.to_path_buf())
    })
}

fn compute_store_key(codex_home: &Path) -> std::io::Result<String> {
    let path_str = codex_home.to_string_lossy();
    let mut hasher = Sha256::new();
    hasher.update(path_str.as_bytes());
    let digest = hasher.finalize();
    let hex = format!("{digest:x}");
    let truncated = hex.get(..16).unwrap_or(&hex);
    Ok(format!("cli|{truncated}"))
}

#[derive(Clone, Debug)]
struct DirectKeyringAuthStorage {
    codex_home: PathBuf,
    keyring_store: Arc<dyn KeyringStore>,
}

impl DirectKeyringAuthStorage {
    fn new(codex_home: PathBuf, keyring_store: Arc<dyn KeyringStore>) -> Self {
        Self {
            codex_home,
            keyring_store,
        }
    }

    fn load_from_keyring(&self, key: &str) -> std::io::Result<Option<AuthAccountStore>> {
        match self.keyring_store.load(KEYRING_SERVICE, key) {
            Ok(Some(serialized)) => deserialize_auth_store(&serialized).map(Some),
            Ok(None) => Ok(None),
            Err(error) => Err(std::io::Error::other(format!(
                "failed to load CLI auth from keyring: {}",
                error.message()
            ))),
        }
    }

    fn save_to_keyring(&self, key: &str, value: &str) -> std::io::Result<()> {
        match self.keyring_store.save(KEYRING_SERVICE, key, value) {
            Ok(()) => Ok(()),
            Err(error) => {
                let message = format!(
                    "failed to write OAuth tokens to keyring: {}",
                    error.message()
                );
                warn!("{message}");
                Err(std::io::Error::other(message))
            }
        }
    }
}

impl AuthStorageBackend for DirectKeyringAuthStorage {
    fn load_store(&self) -> std::io::Result<Option<AuthAccountStore>> {
        let key = compute_store_key(&self.codex_home)?;
        self.load_from_keyring(&key)
    }

    fn load(&self) -> std::io::Result<Option<AuthDotJson>> {
        let key = compute_store_key(&self.codex_home)?;
        match self.keyring_store.load(KEYRING_SERVICE, &key) {
            Ok(Some(serialized)) => serde_json::from_str(&serialized).map(Some).map_err(|err| {
                std::io::Error::other(format!(
                    "failed to deserialize CLI auth from keyring: {err}"
                ))
            }),
            Ok(None) => Ok(None),
            Err(error) => Err(std::io::Error::other(format!(
                "failed to load CLI auth from keyring: {}",
                error.message()
            ))),
        }
    }

    fn save_store(&self, store: &AuthAccountStore) -> std::io::Result<()> {
        store.validate()?;
        let key = compute_store_key(&self.codex_home)?;
        let serialized = serde_json::to_string(store).map_err(std::io::Error::other)?;
        self.save_to_keyring(&key, &serialized)?;
        if let Err(err) = delete_file_if_exists(&self.codex_home) {
            warn!("failed to remove stale CLI auth file after keyring save: {err}");
        }
        Ok(())
    }

    fn delete(&self) -> std::io::Result<bool> {
        let key = compute_store_key(&self.codex_home)?;
        let keyring_removed = self
            .keyring_store
            .delete(KEYRING_SERVICE, &key)
            .map_err(|err| {
                std::io::Error::other(format!("failed to delete auth from keyring: {err}"))
            })?;
        let file_removed = delete_file_if_exists(&self.codex_home)?;
        Ok(keyring_removed || file_removed)
    }
}

#[derive(Clone)]
struct SecretsKeyringAuthStorage {
    codex_home: PathBuf,
    direct_storage: DirectKeyringAuthStorage,
    secrets_manager: SecretsManager,
}

impl Debug for SecretsKeyringAuthStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretsKeyringAuthStorage")
            .field("codex_home", &self.codex_home)
            .finish_non_exhaustive()
    }
}

impl SecretsKeyringAuthStorage {
    fn new(codex_home: PathBuf, keyring_store: Arc<dyn KeyringStore>) -> Self {
        let direct_storage =
            DirectKeyringAuthStorage::new(codex_home.clone(), Arc::clone(&keyring_store));
        let secrets_manager = SecretsManager::new_with_keyring_store_and_namespace(
            codex_home.clone(),
            SecretsBackendKind::Local,
            keyring_store,
            LocalSecretsNamespace::CodexAuth,
        );
        Self {
            codex_home,
            direct_storage,
            secrets_manager,
        }
    }
}

impl AuthStorageBackend for SecretsKeyringAuthStorage {
    fn load_store(&self) -> std::io::Result<Option<AuthAccountStore>> {
        match self
            .secrets_manager
            .get(&SecretScope::Global, &CODEX_AUTH_SECRET_NAME)
            .map_err(|err| {
                std::io::Error::other(format!(
                    "failed to load CLI auth from encrypted auth storage: {err}"
                ))
            })? {
            Some(serialized) => deserialize_auth_store(&serialized).map(Some),
            None => Ok(None),
        }
    }

    fn load(&self) -> std::io::Result<Option<AuthDotJson>> {
        let serialized = self
            .secrets_manager
            .get(&SecretScope::Global, &CODEX_AUTH_SECRET_NAME)
            .map_err(|err| {
                std::io::Error::other(format!(
                    "failed to load CLI auth from encrypted auth storage: {err}"
                ))
            })?;
        serialized
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(std::io::Error::other)
    }

    fn save_store(&self, store: &AuthAccountStore) -> std::io::Result<()> {
        store.validate()?;
        let serialized = serde_json::to_string(store).map_err(std::io::Error::other)?;
        self.secrets_manager
            .set(&SecretScope::Global, &CODEX_AUTH_SECRET_NAME, &serialized)
            .map_err(|err| {
                let message =
                    format!("failed to write OAuth tokens to encrypted auth storage: {err}");
                warn!("{message}");
                std::io::Error::other(message)
            })?;
        if let Err(err) = delete_file_if_exists(&self.codex_home) {
            warn!("failed to remove stale CLI auth file after encrypted auth save: {err}");
        }
        Ok(())
    }

    fn delete(&self) -> std::io::Result<bool> {
        let keyring_removed = self
            .secrets_manager
            .delete(&SecretScope::Global, &CODEX_AUTH_SECRET_NAME)
            .map_err(|err| {
                std::io::Error::other(format!(
                    "failed to delete auth from encrypted auth storage: {err}"
                ))
            })?;
        let file_removed = delete_file_if_exists(&self.codex_home)?;
        let direct_removed = self.direct_storage.delete()?;
        Ok(keyring_removed || file_removed || direct_removed)
    }
}

#[derive(Clone, Debug)]
struct AutoAuthStorage {
    codex_home: PathBuf,
    keyring_storage: Arc<dyn AuthStorageBackend>,
    file_storage: Arc<FileAuthStorage>,
}

impl AutoAuthStorage {
    fn new(
        codex_home: PathBuf,
        keyring_store: Arc<dyn KeyringStore>,
        keyring_backend_kind: AuthKeyringBackendKind,
    ) -> Self {
        Self {
            codex_home: codex_home.clone(),
            keyring_storage: create_keyring_auth_storage(
                codex_home.clone(),
                keyring_store,
                keyring_backend_kind,
            ),
            file_storage: Arc::new(FileAuthStorage::new(codex_home)),
        }
    }

    fn transaction_path(&self) -> PathBuf {
        self.codex_home.join(".auth.json.transaction")
    }

    fn store_hash(store: &AuthAccountStore) -> std::io::Result<String> {
        let mut normalized = store.clone();
        normalized.pending_keyring_write = false;
        let serialized = serde_json::to_vec(&normalized)?;
        Ok(format!("{:x}", Sha256::digest(serialized)))
    }

    fn write_transaction(&self, store: &AuthAccountStore) -> std::io::Result<()> {
        let path = self.transaction_path();
        let parent = path.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "transaction path {} has no parent directory",
                    path.display()
                ),
            )
        })?;
        std::fs::create_dir_all(parent)?;
        let mut temporary = NamedTempFile::new_in(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            temporary
                .as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        temporary.write_all(Self::store_hash(store)?.as_bytes())?;
        temporary.flush()?;
        temporary.as_file().sync_all()?;
        replace_auth_file(temporary, &path)
    }

    fn finish_transaction(&self) {
        if let Err(err) = delete_path_if_exists(&self.transaction_path()) {
            warn!("failed to remove CLI auth transaction marker: {err}");
        }
    }

    fn recover_transaction(&self) -> std::io::Result<Option<AuthAccountStore>> {
        let expected_hash = match std::fs::read_to_string(self.transaction_path()) {
            Ok(hash) => hash,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err),
        };
        let keyring_store = self.keyring_storage.load_store()?;
        if let Some(store) = keyring_store
            && Self::store_hash(&store)? == expected_hash
        {
            match self.file_storage.delete() {
                Ok(_) => self.finish_transaction(),
                Err(err) => {
                    warn!("failed to remove stale CLI auth fallback file: {err}");
                }
            }
            return Ok(Some(store));
        }
        let file_store = self.file_storage.load_store()?;
        if let Some(store) = file_store
            && Self::store_hash(&store)? == expected_hash
        {
            self.finish_transaction();
            return Ok(Some(store));
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unable to resolve interrupted CLI auth transaction",
        ))
    }

    fn load_preferred_store(&self) -> std::io::Result<Option<AuthAccountStore>> {
        if let Some(store) = self.recover_transaction()? {
            return Ok(Some(store));
        }
        let mut file_store = self.file_storage.load_store()?;
        if file_store
            .as_ref()
            .is_some_and(|store| store.pending_keyring_write || store.account_profiles.is_some())
        {
            if let Some(store) = file_store.as_mut() {
                let was_pending = store.pending_keyring_write;
                store.pending_keyring_write = false;
                self.reconcile_keyring_store(
                    store,
                    self.keyring_storage.load_store(),
                    was_pending,
                )?;
            }
            return Ok(file_store);
        }
        match self.keyring_storage.load_store() {
            Ok(Some(store)) => Ok(Some(store)),
            Ok(None) => Ok(file_store),
            Err(err) => {
                warn!("failed to load CLI auth from keyring, falling back to file storage: {err}");
                Ok(file_store)
            }
        }
    }

    fn reconcile_keyring_store(
        &self,
        store: &AuthAccountStore,
        keyring_store: std::io::Result<Option<AuthAccountStore>>,
        file_was_pending: bool,
    ) -> std::io::Result<()> {
        match keyring_store {
            Ok(Some(keyring_store)) if keyring_store == *store => {
                if let Err(err) = self.file_storage.delete() {
                    warn!("failed to remove matching CLI auth fallback file: {err}");
                }
                Ok(())
            }
            Ok(Some(_)) if !file_was_pending => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "conflicting multi-account auth stores exist in file and keyring storage",
            )),
            Ok(_) | Err(_) => Ok(()),
        }
    }

    fn save_store_unlocked(&self, store: &AuthAccountStore) -> std::io::Result<()> {
        self.write_transaction(store)?;
        match self.keyring_storage.save_store(store) {
            Ok(()) => {
                match self.file_storage.delete() {
                    Ok(_) => self.finish_transaction(),
                    Err(err) => {
                        warn!("failed to remove CLI auth fallback file: {err}");
                    }
                }
                Ok(())
            }
            Err(err) => {
                warn!("failed to save auth to keyring, falling back to file storage: {err}");
                let mut pending_store = store.clone();
                pending_store.pending_keyring_write = true;
                self.file_storage.save_store(&pending_store)?;
                self.finish_transaction();
                Ok(())
            }
        }
    }
}

impl AuthStorageBackend for AutoAuthStorage {
    fn load_store(&self) -> std::io::Result<Option<AuthAccountStore>> {
        self.load_preferred_store()
    }

    fn load(&self) -> std::io::Result<Option<AuthDotJson>> {
        match self.load_preferred_store() {
            Ok(store) => Ok(store.map(|store| store.active_auth)),
            Err(err) if self.transaction_path().exists() => Err(err),
            Err(err) if err.kind() == std::io::ErrorKind::InvalidData => Err(err),
            Err(err) => {
                warn!("failed to load complete CLI auth store, falling back to active auth: {err}");
                match self.keyring_storage.load() {
                    Ok(Some(auth)) => Ok(Some(auth)),
                    Ok(None) => self.file_storage.load(),
                    Err(err) => {
                        warn!(
                            "failed to load CLI auth from keyring, falling back to file storage: {err}"
                        );
                        self.file_storage.load()
                    }
                }
            }
        }
    }

    fn save_store(&self, store: &AuthAccountStore) -> std::io::Result<()> {
        self.save_store_unlocked(store)
    }

    fn delete(&self) -> std::io::Result<bool> {
        self.keyring_storage.delete()
    }
}

// A global in-memory store for mapping codex_home -> AuthDotJson.
static EPHEMERAL_AUTH_STORE: Lazy<Mutex<HashMap<String, AuthAccountStore>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Debug)]
struct EphemeralAuthStorage {
    codex_home: PathBuf,
}

impl EphemeralAuthStorage {
    fn new(codex_home: PathBuf) -> Self {
        Self { codex_home }
    }

    fn with_store<F, T>(&self, action: F) -> std::io::Result<T>
    where
        F: FnOnce(&mut HashMap<String, AuthAccountStore>, String) -> std::io::Result<T>,
    {
        let key = compute_store_key(&self.codex_home)?;
        let mut store = EPHEMERAL_AUTH_STORE
            .lock()
            .map_err(|_| std::io::Error::other("failed to lock ephemeral auth storage"))?;
        action(&mut store, key)
    }
}

impl AuthStorageBackend for EphemeralAuthStorage {
    fn load_store(&self) -> std::io::Result<Option<AuthAccountStore>> {
        self.with_store(|store, key| Ok(store.get(&key).cloned()))
    }

    fn save_store(&self, auth: &AuthAccountStore) -> std::io::Result<()> {
        auth.validate()?;
        self.with_store(|store, key| {
            store.insert(key, auth.clone());
            Ok(())
        })
    }

    fn mutate_active(&self, mutation: ActiveAuthMutation) -> std::io::Result<AuthDotJson> {
        self.with_store(|store, key| {
            if !store.contains_key(&key) {
                let seed = mutation
                    .seed()
                    .map(AuthAccountStore::legacy)
                    .ok_or_else(|| std::io::Error::other("auth data is not available"))?;
                store.insert(key.clone(), seed);
            }
            let account_store = store
                .get_mut(&key)
                .ok_or_else(|| std::io::Error::other("auth data is not available"))?;
            mutation.apply(&mut account_store.active_auth)?;
            account_store.validate()?;
            Ok(account_store.active_auth.clone())
        })
    }

    fn save_profile(
        &self,
        metadata: AccountProfileMetadata,
        auth: AuthDotJson,
    ) -> std::io::Result<()> {
        self.with_store(|store, key| {
            let account_store = store
                .get_mut(&key)
                .ok_or_else(|| std::io::Error::other("active auth data is not available"))?;
            account_store.save_profile(metadata, auth)
        })
    }

    fn remove_inactive_profile(&self, profile_id: &str) -> std::io::Result<Option<AuthDotJson>> {
        self.with_store(|store, key| {
            let account_store = store
                .get_mut(&key)
                .ok_or_else(|| std::io::Error::other("active auth data is not available"))?;
            account_store.remove_inactive_profile(profile_id)
        })
    }

    fn delete(&self) -> std::io::Result<bool> {
        self.with_store(|store, key| Ok(store.remove(&key).is_some()))
    }
}

pub(super) fn create_auth_storage(
    codex_home: PathBuf,
    mode: AuthCredentialsStoreMode,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> Arc<dyn AuthStorageBackend> {
    let keyring_store: Arc<dyn KeyringStore> = Arc::new(DefaultKeyringStore);
    create_auth_storage_with_store(codex_home, mode, keyring_store, keyring_backend_kind)
}

fn create_auth_storage_with_store(
    codex_home: PathBuf,
    mode: AuthCredentialsStoreMode,
    keyring_store: Arc<dyn KeyringStore>,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> Arc<dyn AuthStorageBackend> {
    if mode == AuthCredentialsStoreMode::Ephemeral {
        return Arc::new(EphemeralAuthStorage::new(codex_home));
    }
    let backend: Arc<dyn AuthStorageBackend> = match mode {
        AuthCredentialsStoreMode::File => Arc::new(FileAuthStorage::new(codex_home.clone())),
        AuthCredentialsStoreMode::Keyring => {
            create_keyring_auth_storage(codex_home.clone(), keyring_store, keyring_backend_kind)
        }
        AuthCredentialsStoreMode::Auto => Arc::new(AutoAuthStorage::new(
            codex_home.clone(),
            keyring_store,
            keyring_backend_kind,
        )),
        AuthCredentialsStoreMode::Ephemeral => unreachable!("ephemeral mode returned above"),
    };
    Arc::new(SerializedAuthStorage::new(codex_home, backend))
}

fn create_keyring_auth_storage(
    codex_home: PathBuf,
    keyring_store: Arc<dyn KeyringStore>,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> Arc<dyn AuthStorageBackend> {
    match keyring_backend_kind {
        AuthKeyringBackendKind::Direct => {
            Arc::new(DirectKeyringAuthStorage::new(codex_home, keyring_store))
        }
        AuthKeyringBackendKind::Secrets => {
            Arc::new(SecretsKeyringAuthStorage::new(codex_home, keyring_store))
        }
    }
}

#[cfg(test)]
#[path = "multi_account_storage_tests.rs"]
mod multi_account_tests;

#[cfg(test)]
#[path = "storage_tests.rs"]
mod tests;
