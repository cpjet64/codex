use base64::Engine;
use codex_config::types::AuthCredentialsStoreMode;
use codex_protocol::auth::AuthMode;
use pretty_assertions::assert_eq;
use tempfile::tempdir;

use super::AccountProfileMetadata;
use super::AuthAccountStore;
use super::AuthDotJson;
use super::AuthKeyringBackendKind;
use super::login_with_api_key;
use super::login_with_api_key_for_profile;
use super::login_with_bedrock_access_keys_for_profile;
use super::login_with_bedrock_api_key_for_profile;
use super::remove_inactive_profile;

fn profile(id: &str, label: &str) -> AccountProfileMetadata {
    AccountProfileMetadata {
        id: id.to_string(),
        label: label.to_string(),
    }
}

fn read_store(codex_home: &std::path::Path) -> anyhow::Result<AuthAccountStore> {
    let contents = std::fs::read_to_string(codex_home.join("auth.json"))?;
    Ok(serde_json::from_str(&contents)?)
}

fn file_store() -> (AuthCredentialsStoreMode, AuthKeyringBackendKind) {
    (
        AuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::default(),
    )
}

#[test]
fn adding_and_updating_a_profile_preserves_the_active_login() -> anyhow::Result<()> {
    let codex_home = tempdir()?;
    let (mode, keyring) = file_store();
    login_with_api_key(codex_home.path(), "active-key", mode, keyring)?;
    login_with_api_key_for_profile(
        codex_home.path(),
        profile("secondary", "Secondary"),
        "secondary-key",
        mode,
        keyring,
    )?;
    login_with_api_key_for_profile(
        codex_home.path(),
        profile("secondary", "Renamed"),
        "updated-key",
        mode,
        keyring,
    )?;

    let store = read_store(codex_home.path())?;
    assert_eq!(store.active_auth, api_key_auth("active-key"));
    let profiles = store
        .account_profiles
        .expect("profiles should be initialized");
    assert_eq!(profiles.active_profile, profile("legacy", "Account 1"));
    assert_eq!(profiles.inactive_profiles.len(), 1);
    assert_eq!(
        profiles.inactive_profiles[0].metadata,
        profile("secondary", "Renamed")
    );
    assert_eq!(
        profiles.inactive_profiles[0].auth,
        api_key_auth("updated-key")
    );

    let error = login_with_api_key_for_profile(
        codex_home.path(),
        profile("legacy", "Active"),
        "replacement-key",
        mode,
        keyring,
    )
    .expect_err("add-profile login must not replace active auth");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        read_store(codex_home.path())?.active_auth,
        api_key_auth("active-key")
    );
    Ok(())
}

#[test]
fn profile_login_supports_bedrock_auth_methods() -> anyhow::Result<()> {
    let codex_home = tempdir()?;
    let (mode, keyring) = file_store();
    login_with_api_key(codex_home.path(), "active-key", mode, keyring)?;
    login_with_bedrock_api_key_for_profile(
        codex_home.path(),
        profile("bedrock-key", "Bedrock API key"),
        "bedrock-secret",
        "us-east-1",
        mode,
        keyring,
    )?;
    login_with_bedrock_access_keys_for_profile(
        codex_home.path(),
        profile("bedrock-aws", "Bedrock access keys"),
        "access-key",
        "secret-key",
        Some("session-token"),
        mode,
        keyring,
    )?;

    let store = read_store(codex_home.path())?;
    assert_eq!(store.active_auth, api_key_auth("active-key"));
    let profiles = store
        .account_profiles
        .expect("profiles should be initialized");
    assert_eq!(profiles.inactive_profiles.len(), 2);
    assert_eq!(
        profiles.inactive_profiles[0].auth.auth_mode,
        Some(AuthMode::BedrockApiKey)
    );
    assert_eq!(
        profiles.inactive_profiles[1].auth.auth_mode,
        Some(AuthMode::BedrockAccessKeys)
    );
    Ok(())
}

#[test]
fn removing_profiles_rejects_the_active_profile_and_preserves_other_auth() -> anyhow::Result<()> {
    let codex_home = tempdir()?;
    let (mode, keyring) = file_store();
    login_with_api_key(codex_home.path(), "active-key", mode, keyring)?;
    login_with_api_key_for_profile(
        codex_home.path(),
        profile("secondary", "Secondary"),
        "secondary-key",
        mode,
        keyring,
    )?;

    let error = remove_inactive_profile(codex_home.path(), "legacy", mode, keyring)
        .expect_err("active profile removal must fail");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(
        read_store(codex_home.path())?.active_auth,
        api_key_auth("active-key")
    );

    let removed = remove_inactive_profile(codex_home.path(), "secondary", mode, keyring)?;
    assert_eq!(removed, Some(api_key_auth("secondary-key")));
    let store = read_store(codex_home.path())?;
    assert_eq!(store.active_auth, api_key_auth("active-key"));
    assert!(
        store
            .account_profiles
            .expect("profiles should remain initialized")
            .inactive_profiles
            .is_empty()
    );
    Ok(())
}

#[tokio::test]
async fn oauth_profile_persistence_is_atomic_with_the_active_login() -> anyhow::Result<()> {
    let codex_home = tempdir()?;
    let (mode, keyring) = file_store();
    login_with_api_key(codex_home.path(), "active-key", mode, keyring)?;
    let before = std::fs::read(codex_home.path().join("auth.json"))?;

    let error = crate::server::persist_tokens_for_profile_async(
        codex_home.path(),
        profile("oauth", "OAuth"),
        "invalid-token".to_string(),
        "access".to_string(),
        "refresh".to_string(),
        mode,
        keyring,
    )
    .await
    .expect_err("invalid OAuth tokens must not mutate auth storage");
    assert_eq!(error.kind(), std::io::ErrorKind::Other);
    assert_eq!(std::fs::read(codex_home.path().join("auth.json"))?, before);

    crate::server::persist_tokens_for_profile_async(
        codex_home.path(),
        profile("oauth", "OAuth"),
        chatgpt_id_token(),
        "access".to_string(),
        "refresh".to_string(),
        mode,
        keyring,
    )
    .await?;
    let store = read_store(codex_home.path())?;
    assert_eq!(store.active_auth, api_key_auth("active-key"));
    let oauth = &store
        .account_profiles
        .expect("profiles should be initialized")
        .inactive_profiles[0]
        .auth;
    assert_eq!(oauth.auth_mode, Some(AuthMode::Chatgpt));
    assert_eq!(
        oauth
            .tokens
            .as_ref()
            .map(|tokens| tokens.refresh_token.as_str()),
        Some("refresh")
    );
    Ok(())
}

fn chatgpt_id_token() -> String {
    let encode = |value: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(value);
    let header = serde_json::to_vec(&serde_json::json!({"alg": "none", "typ": "JWT"}))
        .expect("JWT header should serialize");
    let payload = serde_json::to_vec(&serde_json::json!({
        "email": "profile@example.com",
        "https://api.openai.com/auth": {
            "chatgpt_plan_type": "pro",
            "chatgpt_account_id": "profile-account"
        }
    }))
    .expect("JWT payload should serialize");
    format!(
        "{}.{}.{}",
        encode(&header),
        encode(&payload),
        encode(b"sig")
    )
}

fn api_key_auth(api_key: &str) -> AuthDotJson {
    AuthDotJson {
        auth_mode: Some(AuthMode::ApiKey),
        openai_api_key: Some(api_key.to_string()),
        tokens: None,
        last_refresh: None,
        agent_identity: None,
        personal_access_token: None,
        bedrock_api_key: None,
        bedrock_access_keys: None,
    }
}
