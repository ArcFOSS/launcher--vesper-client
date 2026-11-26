//! The backend command worker.
//!
//! A dedicated OS thread runs a current-thread tokio runtime that owns the
//! [`BackendCommand`] receiver and drives the domain managers. Every outcome is
//! reported back through the shared [`UiUpdate`] slot that the views poll.

use std::sync::{Arc, Mutex};

use anyhow::Context as _;
use open::that;
use tokio::sync::mpsc;
use vesper_core::auth::flow::AuthManager;
use vesper_core::auth::models::{AccountType, AuthProfile, StoredTokens};
use vesper_core::bridge::commands::{BackendCommand, ModSource};
use vesper_core::bridge::updates::{InstalledModInfo, UiUpdate};
use vesper_core::config::{data_dir, VesperConfig};
use vesper_core::instances::InstanceManager;
use vesper_core::mods::ModManager;

/// Microsoft's public client id for the Minecraft launcher.
pub const MS_CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb";

/// Owns the command channel for the lifetime of the backend thread.
pub fn spawn(cmds: mpsc::Receiver<BackendCommand>, updates: Arc<Mutex<Option<UiUpdate>>>) {
    std::thread::Builder::new()
        .name("vesper-backend".to_string())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build the backend runtime");

            runtime.block_on(async move {
                let mut cmds = cmds;
                while let Some(cmd) = cmds.recv().await {
                    handle_command(cmd, &updates).await;
                }
            });
        })
        .expect("failed to spawn the backend thread");
}

async fn handle_command(cmd: BackendCommand, updates: &Arc<Mutex<Option<UiUpdate>>>) {
    match cmd {
        BackendCommand::AuthStartBrowserLogin { .. } => {
            if let Err(error) = run_auth_flow(updates).await {
                set_update(
                    updates,
                    UiUpdate::AuthError {
                        message: format!("Auth failed: {error:#}"),
                    },
                );
            }
        }
        BackendCommand::AuthGetStatus => {
            let session = load_session();
            set_update(
                updates,
                UiUpdate::AuthStatus {
                    is_logged_in: session.is_some(),
                    profile: session.as_ref().map(|(profile, _)| profile.clone()),
                },
            );
        }
        BackendCommand::AuthLogout => {
            set_update(updates, UiUpdate::AuthLoggedOut);
        }
        BackendCommand::InstanceCreate {
            name,
            mc_version,
            loader,
            loader_version,
        } => {
            let Ok(manager) = InstanceManager::new() else {
                set_update(
                    updates,
                    UiUpdate::InstanceError {
                        message: "Could not open the instance directory.".into(),
                    },
                );
                return;
            };
            match manager.create(&name, &mc_version, loader, loader_version) {
                Ok(instance) => {
                    set_update(
                        updates,
                        UiUpdate::InstanceCreated {
                            id: instance.id.clone(),
                            name: instance.config.name.clone(),
                        },
                    );
                    set_update(updates, list_instances(&manager));
                }
                Err(error) => {
                    set_update(
                        updates,
                        UiUpdate::InstanceError {
                            message: format!("Instance creation failed: {error}"),
                        },
                    );
                }
            }
        }
        BackendCommand::InstanceList => {
            let Ok(manager) = InstanceManager::new() else {
                return;
            };
            set_update(updates, list_instances(&manager));
        }
        BackendCommand::InstanceDelete { id } => {
            let Ok(manager) = InstanceManager::new() else {
                return;
            };
            if manager.delete(&id).is_ok() {
                set_update(updates, UiUpdate::InstanceDeleted { id });
                set_update(updates, list_instances(&manager));
            } else {
                set_update(
                    updates,
                    UiUpdate::InstanceError {
                        message: format!("Could not delete instance {id}."),
                    },
                );
            }
        }
        BackendCommand::InstanceRename { id, name } => {
            let Ok(manager) = InstanceManager::new() else {
                return;
            };
            if manager.rename(&id, &name).is_ok() {
                set_update(updates, UiUpdate::InstanceRenamed { id });
                set_update(updates, list_instances(&manager));
            } else {
                set_update(
                    updates,
                    UiUpdate::InstanceError {
                        message: format!("Could not rename instance {id}."),
                    },
                );
            }
        }
        BackendCommand::ModSearch { query, source } => {
            let Ok(instance_manager) = InstanceManager::new() else {
                set_update(
                    updates,
                    UiUpdate::ModError {
                        message: "Could not start the mod manager.".into(),
                    },
                );
                return;
            };
            let Ok(manager) = ModManager::new(instance_manager, None) else {
                set_update(
                    updates,
                    UiUpdate::ModError {
                        message: "Could not start the mod manager.".into(),
                    },
                );
                return;
            };
            match manager.search(&query, source).await {
                Ok(mods) => set_update(updates, UiUpdate::ModSearchResults(mods)),
                Err(error) => set_update(
                    updates,
                    UiUpdate::ModError {
                        message: format!("Mod search failed: {error}"),
                    },
                ),
            }
        }
        BackendCommand::ModDownload {
            instance_id,
            project_id,
            file_id,
            file_name,
            source,
        } => {
            let Ok(instance_manager) = InstanceManager::new() else {
                set_update(
                    updates,
                    UiUpdate::ModError {
                        message: "Could not start the mod manager.".into(),
                    },
                );
                return;
            };
            let Ok(manager) = ModManager::new(instance_manager, None) else {
                set_update(
                    updates,
                    UiUpdate::ModError {
                        message: "Could not start the mod manager.".into(),
                    },
                );
                return;
            };
            let result = manager
                .download_mod(&instance_id, &project_id, &file_id, &file_name, source)
                .await;
            if let Err(error) = result {
                set_update(
                    updates,
                    UiUpdate::ModError {
                        message: format!("Mod download failed: {error}"),
                    },
                );
            } else {
                set_update(
                    updates,
                    UiUpdate::ModDownloadComplete {
                        project_name: file_name,
                    },
                );
            }
        }
        BackendCommand::ModListInstalled { instance_id } => {
            let Ok(instance_manager) = InstanceManager::new() else {
                return;
            };
            let Ok(manager) = ModManager::new(instance_manager, None) else {
                return;
            };
            match manager.list_installed(&instance_id) {
                Ok(paths) => {
                    let mods = paths
                        .into_iter()
                        .filter_map(|path| {
                            let name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_string();
                            Some(InstalledModInfo {
                                name,
                                version: String::new(),
                                source: ModSource::Modrinth.to_string(),
                            })
                        })
                        .collect();
                    set_update(updates, UiUpdate::ModListResult(mods));
                }
                Err(error) => {
                    set_update(
                        updates,
                        UiUpdate::ModError {
                            message: format!("Could not list installed mods: {error}"),
                        },
                    );
                }
            }
        }
        _ => {}
    }
}

fn list_instances(manager: &InstanceManager) -> UiUpdate {
    UiUpdate::InstanceListResult(manager.list().unwrap_or_default())
}

/// Where the signed-in session's tokens live: the app data dir, as TOML.
fn tokens_path() -> anyhow::Result<std::path::PathBuf> {
    let dir = data_dir().context("Failed to resolve the data directory")?;
    Ok(dir.join("auth_tokens.toml"))
}

/// Persists the signed-in user and their tokens: the profile lands in the
/// app config (which is the source of truth for who is signed in), the tokens
/// land next to it.
fn persist_session(profile: &AuthProfile, tokens: &StoredTokens) -> anyhow::Result<()> {
    let mut config = VesperConfig::load().context("Failed to load the app config")?;
    if let Some(existing) = config.profiles.iter_mut().find(|p| p.id == profile.id) {
        *existing = profile.clone();
    } else {
        config.profiles.push(profile.clone());
    }
    config.set_active_profile(&profile.id);
    config.save().context("Failed to save the app config")?;

    let path = tokens_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create the auth directory")?;
    }
    let contents = toml::to_string_pretty(tokens).context("Failed to serialize the auth tokens")?;
    std::fs::write(&path, contents).context("Failed to write the auth tokens")?;
    Ok(())
}

/// Reads the persisted session back, if there is one.
fn load_session() -> Option<(AuthProfile, StoredTokens)> {
    let path = tokens_path().ok()?;
    let contents = std::fs::read_to_string(&path).ok()?;
    let tokens: StoredTokens = toml::from_str(&contents).ok()?;
    if tokens.access_token.is_empty() {
        return None;
    }
    let mc_id = tokens.mc_profile.as_ref().map(|m| m.id.clone())?;
    let config = VesperConfig::load().ok()?;
    let profile = config.profiles.iter().find(|p| p.id == mc_id).cloned()?;
    Some((profile, tokens))
}

pub async fn run_auth_flow(updates: &Arc<Mutex<Option<UiUpdate>>>) -> anyhow::Result<()> {
    let auth_manager = AuthManager::new(MS_CLIENT_ID.to_string())
        .context("Failed to initialize auth manager")?;

    let (auth_url, csrf_state, pkce_verifier, port) = auth_manager
        .start_browser_login()
        .await
        .context("Failed to start browser login")?;

    that(auth_url.as_str()).context("Failed to open browser")?;

    let tokens = auth_manager
        .complete_browser_login(port, &pkce_verifier, &csrf_state)
        .await
        .context("Browser login completion failed")?;

    let mc_profile = tokens
        .mc_profile
        .as_ref()
        .context("No Minecraft profile in auth response")?;

    let profile = AuthProfile {
        id: mc_profile.id.clone(),
        display_name: mc_profile.name.clone(),
        email: None,
        account_type: AccountType::Java,
        minecraft_username: Some(mc_profile.name.clone()),
        minecraft_uuid: Some(mc_profile.id.clone()),
        xbox_gamertag: None,
        bedrock_profile: None,
        device_keys: None,
        identity_keys: None,
    };

    persist_session(&profile, &tokens).context("Failed to persist the auth session")?;

    set_update(updates, UiUpdate::AuthSuccess { profile });
    Ok(())
}

pub fn set_update(target: &Arc<Mutex<Option<UiUpdate>>>, update: UiUpdate) {
    if let Ok(mut guard) = target.lock() {
        *guard = Some(update);
    }
}
