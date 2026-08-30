use super::*;
use crate::auth::AccountProfileMetadata;
use crate::auth::AccountProfiles;
use crate::auth::InactiveAccountProfile;
use crate::token_data::IdTokenInfo;
use crate::token_data::TokenData;
use codex_keyring_store::tests::MockKeyringStore;
use codex_protocol::auth::AuthMode;
use keyring::Error as KeyringError;
use pretty_assertions::assert_eq;
use std::sync::Arc;
use tempfile::tempdir;

fn api_key_auth(key: &str) -> AuthDotJson {
    AuthDotJson {
        auth_mode: Some(AuthMode::ApiKey),
        openai_api_key: Some(key.to_string()),
        tokens: None,
        last_refresh: None,
        agent_identity: None,
        personal_access_token: None,
        bedrock_api_key: None,
        bedrock_access_keys: None,
    }
}

fn metadata(id: &str, label: &str) -> AccountProfileMetadata {
    AccountProfileMetadata {
        id: id.to_string(),
        label: label.to_string(),
    }
}

fn account_store() -> AuthAccountStore {
    AuthAccountStore {
        active_auth: api_key_auth("active-key"),
        account_profiles: Some(AccountProfiles {
            version: 1,
            active_profile: metadata("p", "Personal"),
            inactive_profiles: vec![InactiveAccountProfile {
                metadata: metadata("w", "Work"),
                auth: api_key_auth("inactive-key"),
            }],
        }),
        pending_keyring_write: false,
    }
}

fn token_data() -> TokenData {
    TokenData {
        id_token: IdTokenInfo {
            raw_jwt: "eyJhbGciOiJub25lIn0.eyJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnt9fQ.eA"
                .to_string(),
            ..Default::default()
        },
        access_token: "old-access".to_string(),
        refresh_token: "old-refresh".to_string(),
        account_id: None,
    }
}

#[test]
fn legacy_auth_loads_without_rewriting_the_schema() -> anyhow::Result<()> {
    let codex_home = tempdir()?;
    let storage = FileAuthStorage::new(codex_home.path().to_path_buf());
    let legacy_auth = api_key_auth("legacy-key");
    let auth_file = get_auth_file(codex_home.path());
    let original = serde_json::to_vec_pretty(&legacy_auth)?;
    std::fs::write(&auth_file, &original)?;
    assert_eq!(
        storage.load_store()?,
        Some(AuthAccountStore::legacy(legacy_auth))
    );
    assert_eq!(std::fs::read(auth_file)?, original);
    Ok(())
}

#[test]
fn multi_account_document_remains_readable_as_legacy_auth() -> anyhow::Result<()> {
    let codex_home = tempdir()?;
    let storage = FileAuthStorage::new(codex_home.path().to_path_buf());
    let store = account_store();
    storage.save_store(&store)?;
    assert_eq!(storage.load()?, Some(store.active_auth));
    Ok(())
}

#[test]
fn serialized_storage_tolerates_invalid_inactive_profile_metadata_on_active_load()
-> anyhow::Result<()> {
    let codex_home = tempdir()?;
    let auth_file = get_auth_file(codex_home.path());
    let mut invalid = serde_json::to_value(account_store())?;
    invalid["account_profiles"]["version"] = serde_json::json!(2);
    std::fs::write(auth_file, serde_json::to_vec_pretty(&invalid)?)?;
    let storage = create_auth_storage(
        codex_home.path().to_path_buf(),
        AuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::Direct,
    );
    assert_eq!(storage.load()?, Some(api_key_auth("active-key")));
    Ok(())
}

#[test]
fn profile_metadata_validation_rejects_invalid_values() -> anyhow::Result<()> {
    let codex_home = tempdir()?;
    let storage = FileAuthStorage::new(codex_home.path().to_path_buf());
    for (id, label) in [("", "Personal"), ("personal", " ")] {
        let mut store = account_store();
        store
            .account_profiles
            .as_mut()
            .expect("profiles")
            .active_profile = metadata(id, label);
        let invalid = storage
            .save_store(&store)
            .expect_err("invalid metadata should fail validation");
        assert_eq!(invalid.kind(), std::io::ErrorKind::InvalidData);
    }
    let mut duplicate = account_store();
    duplicate
        .account_profiles
        .as_mut()
        .expect("test store should include profiles")
        .inactive_profiles[0]
        .metadata
        .id = "p".to_string();
    let error = storage
        .save_store(&duplicate)
        .expect_err("duplicate profile ids should fail validation");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    Ok(())
}

#[test]
fn active_replacement_does_not_overwrite_invalid_profile_data() -> anyhow::Result<()> {
    let codex_home = tempdir()?;
    let storage = FileAuthStorage::new(codex_home.path().to_path_buf());
    let auth_file = get_auth_file(codex_home.path());
    let mut invalid = serde_json::to_value(account_store())?;
    invalid["account_profiles"]["version"] = serde_json::json!(2);
    let original = serde_json::to_vec_pretty(&invalid)?;
    std::fs::write(&auth_file, &original)?;
    let error = storage
        .update_active(&api_key_auth("replacement-key"))
        .expect_err("invalid profile data should be protected from overwrite");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(std::fs::read(auth_file)?, original);
    Ok(())
}

#[test]
fn all_non_file_backends_preserve_profiles_on_active_update() -> anyhow::Result<()> {
    let direct_home = tempdir()?;
    let ephemeral_home = tempdir()?;
    let secrets_home = tempdir()?;
    let storages: Vec<Box<dyn AuthStorageBackend>> = vec![
        Box::new(DirectKeyringAuthStorage::new(
            direct_home.path().to_path_buf(),
            Arc::new(MockKeyringStore::default()),
        )),
        Box::new(EphemeralAuthStorage::new(
            ephemeral_home.path().to_path_buf(),
        )),
        Box::new(SecretsKeyringAuthStorage::new(
            secrets_home.path().to_path_buf(),
            Arc::new(MockKeyringStore::default()),
        )),
    ];
    for storage in storages {
        let mut expected = account_store();
        storage.save_store(&expected)?;
        expected.active_auth = api_key_auth("replacement-key");
        storage.update_active(&expected.active_auth)?;
        assert_eq!(storage.load_store()?, Some(expected));
    }
    Ok(())
}

#[test]
fn auto_recovers_pending_file_store_over_stale_keyring_data() -> anyhow::Result<()> {
    for mut expected in [
        AuthAccountStore::legacy(api_key_auth("updated-key")),
        account_store(),
    ] {
        let codex_home = tempdir()?;
        let storage = AutoAuthStorage::new(
            codex_home.path().to_path_buf(),
            Arc::new(MockKeyringStore::default()),
            AuthKeyringBackendKind::Direct,
        );
        storage.keyring_storage.save_store(&account_store())?;
        expected.active_auth = api_key_auth("updated-key");
        let mut pending = expected.clone();
        pending.pending_keyring_write = true;
        storage.file_storage.save_store(&pending)?;
        storage.update_active(&expected.active_auth)?;
        assert_eq!(storage.load_store()?, Some(expected));
        assert!(!get_auth_file(codex_home.path()).exists());
    }

    let codex_home = tempdir()?;
    let storage = AutoAuthStorage::new(
        codex_home.path().to_path_buf(),
        Arc::new(MockKeyringStore::default()),
        AuthKeyringBackendKind::Direct,
    );
    let file_store = account_store();
    let mut keyring_store = account_store();
    keyring_store.active_auth = api_key_auth("other-key");
    storage.keyring_storage.save_store(&keyring_store)?;
    storage.file_storage.save_store(&file_store)?;
    let error = storage
        .load_store()
        .expect_err("unmarked conflicting stores must not be reconciled");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    let error = storage
        .load()
        .expect_err("active auth loading must not bypass a store conflict");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(storage.file_storage.load_store()?, Some(file_store));
    assert_eq!(storage.keyring_storage.load_store()?, Some(keyring_store));
    Ok(())
}

#[test]
fn auto_recovers_interrupted_keyring_transaction_by_store_hash() -> anyhow::Result<()> {
    let codex_home = tempdir()?;
    let storage = AutoAuthStorage::new(
        codex_home.path().to_path_buf(),
        Arc::new(MockKeyringStore::default()),
        AuthKeyringBackendKind::Direct,
    );
    let intended = account_store();
    let stale = AuthAccountStore::legacy(api_key_auth("stale-key"));
    storage.file_storage.save_store(&stale)?;
    storage.write_transaction(&intended)?;
    storage.keyring_storage.save_store(&intended)?;

    assert_eq!(storage.load_store()?, Some(intended.clone()));
    assert!(!storage.transaction_path().exists());
    assert!(!get_auth_file(codex_home.path()).exists());

    let fallback_home = tempdir()?;
    let fallback_storage = AutoAuthStorage::new(
        fallback_home.path().to_path_buf(),
        Arc::new(MockKeyringStore::default()),
        AuthKeyringBackendKind::Direct,
    );
    fallback_storage.write_transaction(&intended)?;
    let mut pending = intended;
    pending.pending_keyring_write = true;
    fallback_storage.file_storage.save_store(&pending)?;
    assert_eq!(fallback_storage.load_store()?, Some(pending));
    assert!(!fallback_storage.transaction_path().exists());
    Ok(())
}

#[test]
fn auto_preserves_profiles_across_keyring_failure_and_recovery() -> anyhow::Result<()> {
    let codex_home = tempdir()?;
    let failing_keyring = MockKeyringStore::default();
    failing_keyring.set_error(
        &compute_store_key(codex_home.path())?,
        KeyringError::Invalid("error".into(), "save".into()),
    );
    let failing_storage = AutoAuthStorage::new(
        codex_home.path().to_path_buf(),
        Arc::new(failing_keyring),
        AuthKeyringBackendKind::Direct,
    );
    let expected = account_store();
    failing_storage.save_store(&expected)?;
    let mut pending = expected.clone();
    pending.pending_keyring_write = true;
    assert_eq!(failing_storage.file_storage.load_store()?, Some(pending));

    let recovered = AutoAuthStorage::new(
        codex_home.path().to_path_buf(),
        Arc::new(MockKeyringStore::default()),
        AuthKeyringBackendKind::Direct,
    );
    recovered.update_active(&expected.active_auth)?;
    assert_eq!(recovered.load_store()?, Some(expected));
    assert!(!get_auth_file(codex_home.path()).exists());
    Ok(())
}

#[test]
fn serialized_mutations_wait_for_the_os_lock_and_preserve_updates() -> anyhow::Result<()> {
    let codex_home = tempdir()?;
    let backend = Arc::new(FileAuthStorage::new(codex_home.path().to_path_buf()));
    let storage = SerializedAuthStorage::new(codex_home.path().to_path_buf(), backend);
    let mut seed = account_store();
    seed.active_auth.tokens = Some(token_data());
    storage.save_store(&seed)?;
    let guard = storage.lock()?;
    let canonical_home = canonical_storage_home(codex_home.path());
    let lock_path = canonical_home
        .parent()
        .expect("tempdir should have parent")
        .join(format!(
            ".codex-auth-{}.lock",
            compute_store_key(&canonical_home)?.replace('|', "-")
        ));
    let competing_file = std::fs::File::options().write(true).open(lock_path)?;
    assert!(matches!(
        competing_file.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));

    let (ready_sender, ready_receiver) = std::sync::mpsc::channel();
    let (result_sender, result_receiver) = std::sync::mpsc::channel();
    let mut workers = Vec::new();
    for mutation in [
        ActiveAuthMutation::Tokens {
            id_token: None,
            access_token: Some("new-access".to_string()),
            refresh_token: None,
            refreshed_at: chrono::Utc::now(),
        },
        ActiveAuthMutation::Tokens {
            id_token: None,
            access_token: None,
            refresh_token: Some("new-refresh".to_string()),
            refreshed_at: chrono::Utc::now(),
        },
    ] {
        let storage = create_auth_storage(
            codex_home.path().to_path_buf(),
            AuthCredentialsStoreMode::File,
            AuthKeyringBackendKind::Direct,
        );
        let ready_sender = ready_sender.clone();
        let result_sender = result_sender.clone();
        workers.push(std::thread::spawn(move || {
            ready_sender.send(()).expect("auth mutation ready signal");
            result_sender.send(storage.mutate_active(mutation).map(|_| ()))
        }));
    }
    for _ in 0..2 {
        ready_receiver.recv()?;
    }
    drop(guard);
    for _ in 0..2 {
        result_receiver.recv()?.map_err(anyhow::Error::from)?;
    }
    for worker in workers {
        worker.join().expect("auth mutation worker")?;
    }
    let stored = storage.load_store()?.expect("stored auth");
    let tokens = stored.active_auth.tokens.expect("updated tokens");
    assert_eq!(tokens.access_token, "new-access");
    assert_eq!(tokens.refresh_token, "new-refresh");
    assert!(stored.account_profiles == seed.account_profiles);
    Ok(())
}

#[test]
fn serialized_storage_uses_one_lock_for_equivalent_home_paths() -> anyhow::Result<()> {
    let codex_home = tempdir()?;
    let direct = SerializedAuthStorage::new(
        codex_home.path().to_path_buf(),
        Arc::new(FileAuthStorage::new(codex_home.path().to_path_buf())),
    );
    let aliased_home = codex_home.path().join(".");
    let aliased = SerializedAuthStorage::new(
        aliased_home.clone(),
        Arc::new(FileAuthStorage::new(aliased_home)),
    );
    let guard = direct.lock()?;
    let canonical_home = canonical_storage_home(codex_home.path());
    let lock_path = canonical_home
        .parent()
        .expect("tempdir should have parent")
        .join(format!(
            ".codex-auth-{}.lock",
            compute_store_key(&canonical_home)?.replace('|', "-")
        ));
    let competing_file = std::fs::File::options().write(true).open(lock_path)?;
    assert!(matches!(
        competing_file.try_lock(),
        Err(std::fs::TryLockError::WouldBlock)
    ));
    drop(guard);
    let _aliased_guard = aliased.lock()?;
    Ok(())
}
