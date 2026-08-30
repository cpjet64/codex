use super::ActiveAuthMutation;
use super::AuthStorageBackend;
use super::canonical_storage_home;
use super::compute_store_key;
use crate::auth::AccountProfileMetadata;
use crate::auth::AuthAccountStore;
use crate::auth::AuthDotJson;
use once_cell::sync::Lazy;
use std::fs::File;
use std::fs::OpenOptions;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

static PROCESS_AUTH_SAVE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub(super) struct AuthStorageGuard {
    _process_guard: MutexGuard<'static, ()>,
    _file_guard: File,
}

#[derive(Clone, Debug)]
pub(super) struct SerializedAuthStorage {
    codex_home: PathBuf,
    backend: Arc<dyn AuthStorageBackend>,
}

impl SerializedAuthStorage {
    pub(super) fn new(codex_home: PathBuf, backend: Arc<dyn AuthStorageBackend>) -> Self {
        Self {
            codex_home,
            backend,
        }
    }

    pub(super) fn lock(&self) -> std::io::Result<AuthStorageGuard> {
        let process_guard = PROCESS_AUTH_SAVE_LOCK
            .lock()
            .map_err(|_| std::io::Error::other("failed to lock auth storage"))?;
        let canonical_home = canonical_storage_home(&self.codex_home);
        let lock_parent = canonical_home.parent().unwrap_or(&canonical_home);
        let lock_path = lock_parent.join(format!(
            ".codex-auth-{}.lock",
            compute_store_key(&canonical_home)?.replace('|', "-")
        ));
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(false);
        #[cfg(unix)]
        options.mode(0o600);
        let file_guard = options.open(lock_path)?;
        file_guard.lock()?;
        Ok(AuthStorageGuard {
            _process_guard: process_guard,
            _file_guard: file_guard,
        })
    }
}

impl AuthStorageBackend for SerializedAuthStorage {
    fn load(&self) -> std::io::Result<Option<AuthDotJson>> {
        let _guard = self.lock()?;
        self.backend.load()
    }

    fn load_store(&self) -> std::io::Result<Option<AuthAccountStore>> {
        let _guard = self.lock()?;
        self.backend.load_store()
    }

    fn save_store(&self, store: &AuthAccountStore) -> std::io::Result<()> {
        let _guard = self.lock()?;
        self.backend.save_store(store)
    }

    fn delete(&self) -> std::io::Result<bool> {
        let _guard = self.lock()?;
        self.backend.delete()
    }

    fn mutate_active(&self, mutation: ActiveAuthMutation) -> std::io::Result<AuthDotJson> {
        let _guard = self.lock()?;
        self.backend.mutate_active(mutation)
    }

    fn save_profile(
        &self,
        metadata: AccountProfileMetadata,
        auth: AuthDotJson,
    ) -> std::io::Result<()> {
        let _guard = self.lock()?;
        self.backend.save_profile(metadata, auth)
    }

    fn remove_inactive_profile(&self, profile_id: &str) -> std::io::Result<Option<AuthDotJson>> {
        let _guard = self.lock()?;
        self.backend.remove_inactive_profile(profile_id)
    }
}
