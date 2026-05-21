mod ai;
mod db;
mod import;
mod migration;
mod portforward;
mod r2;
mod s3;
mod sftp;
mod snippets;
mod ssh;
pub mod telemetry;
mod types;
mod vault;

use db::HostDb;
use portforward::manager::PortForwardManager;
use r2::R2Manager;
use s3::transfer_manager::S3TransferManager;
use s3::S3Manager;
use sftp::transfer_manager::TransferManager;
use sftp::SftpManager;
use ssh::manager::SshManager;
use std::sync::Arc;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter("retoom=debug,retoom_lib=debug,russh=info")
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("could not resolve app data dir: {e}"))?;

            if let Err(err) = migration::migrate_legacy_app_state(&app_data_dir) {
                tracing::warn!(
                    error = %err,
                    "failed to migrate legacy anySCP app state; continuing with Retoom state"
                );
            }

            let host_db = HostDb::new(&app_data_dir)
                .map_err(|e| format!("failed to initialise database: {e}"))?;

            migrate_legacy_vault_entries(&host_db);

            app.manage(Arc::new(host_db));

            // SftpManager must be created inside setup so it can be shared with
            // TransferManager, which also needs the AppHandle.
            let sftp_manager = Arc::new(SftpManager::new());
            let transfer_manager = Arc::new(TransferManager::new(
                sftp_manager.clone(),
                app.handle().clone(),
            ));
            app.manage(sftp_manager);
            app.manage(transfer_manager);

            let pf_manager = Arc::new(PortForwardManager::new(app.handle().clone()));
            app.manage(pf_manager);

            let s3_manager = Arc::new(S3Manager::new());
            let s3_transfer_manager = Arc::new(S3TransferManager::new(
                s3_manager.clone(),
                app.handle().clone(),
            ));
            app.manage(s3_manager);
            app.manage(s3_transfer_manager);

            let r2_manager = Arc::new(
                R2Manager::new().map_err(|e| format!("failed to initialise R2 client: {e}"))?,
            );
            app.manage(r2_manager);

            Ok(())
        })
        .manage(SshManager::new())
        .invoke_handler(tauri::generate_handler![
            // SFTP — session & filesystem
            sftp::commands::sftp_open,
            sftp::commands::sftp_close,
            sftp::commands::sftp_list_dir,
            sftp::commands::sftp_home_dir,
            sftp::commands::sftp_mkdir,
            sftp::commands::sftp_create_file,
            sftp::commands::sftp_delete,
            sftp::commands::sftp_rename,
            // SFTP — copy / move
            sftp::commands::sftp_move_entries,
            sftp::commands::sftp_copy_entries,
            // SFTP — legacy direct transfers (kept for VS Code edit workflow)
            sftp::commands::sftp_download,
            sftp::commands::sftp_upload,
            sftp::commands::sftp_cancel_transfer,
            sftp::commands::sftp_edit_in_vscode,
            // SFTP — queue-based Transfer Manager
            sftp::commands::sftp_enqueue_upload,
            sftp::commands::sftp_enqueue_download,
            sftp::commands::sftp_retry_transfer,
            sftp::commands::sftp_list_transfers,
            sftp::commands::sftp_clear_finished_transfers,
            sftp::commands::sftp_set_concurrency,
            // SSH
            ssh::commands::ssh_connect,
            ssh::commands::ssh_split_session,
            ssh::commands::ssh_disconnect,
            ssh::commands::ssh_send_input,
            ssh::commands::ssh_resize_pty,
            ssh::commands::list_ssh_keys,
            ssh::commands::inspect_ssh_key,
            ssh::commands::connect_saved_host,
            ssh::commands::connect_saved_host_no_pty,
            // Host persistence
            db::commands::save_host,
            db::commands::list_hosts,
            db::commands::delete_host,
            db::commands::get_host,
            // Host groups
            db::commands::create_group,
            db::commands::update_group,
            db::commands::list_groups,
            db::commands::delete_group,
            db::commands::delete_group_with_hosts,
            // Connection history
            db::commands::record_connection,
            db::commands::list_recent_connections,
            // Connection history (full audit)
            db::commands::list_connection_history,
            // App settings
            db::commands::save_setting,
            db::commands::load_all_settings,
            // Credential vault
            vault::vault_save_credential,
            vault::vault_delete_credential,
            vault::vault_has_credential,
            // S3
            s3::commands::s3_connect,
            s3::commands::s3_disconnect,
            s3::commands::s3_list_buckets,
            s3::commands::s3_switch_bucket,
            s3::commands::s3_list_objects,
            s3::commands::s3_delete_object,
            s3::commands::s3_delete_objects,
            s3::commands::s3_create_folder,
            s3::commands::s3_presign_url,
            s3::commands::s3_head_object,
            s3::commands::s3_upload_file,
            s3::commands::s3_download_file,
            s3::commands::s3_save_connection,
            s3::commands::s3_list_connections,
            s3::commands::s3_delete_connection,
            s3::commands::s3_reconnect,
            s3::commands::s3_update_connection,
            s3::commands::s3_create_file,
            s3::commands::s3_upload_files,
            s3::commands::s3_delete_prefix,
            // S3 — Transfer Manager
            s3::commands::s3_enqueue_upload,
            s3::commands::s3_enqueue_download,
            s3::commands::s3_cancel_transfer,
            s3::commands::s3_retry_transfer,
            s3::commands::s3_list_transfers,
            s3::commands::s3_clear_finished_transfers,
            s3::commands::s3_edit_in_vscode,
            // Cloudflare R2 management
            r2::r2_list_buckets,
            r2::r2_get_bucket,
            r2::r2_create_bucket,
            r2::r2_patch_bucket,
            r2::r2_delete_bucket,
            r2::r2_get_cors,
            r2::r2_put_cors,
            r2::r2_delete_cors,
            r2::r2_get_lifecycle,
            r2::r2_put_lifecycle,
            r2::r2_delete_lifecycle,
            r2::r2_get_managed_domain,
            r2::r2_update_managed_domain,
            r2::r2_list_custom_domains,
            r2::r2_attach_custom_domain,
            r2::r2_update_custom_domain,
            r2::r2_delete_custom_domain,
            r2::r2_get_metrics,
            // SSH config import
            import::commands::import_parse_ssh_config,
            import::commands::import_save_ssh_hosts,
            // Port forwarding
            portforward::commands::pf_create_rule,
            portforward::commands::pf_update_rule,
            portforward::commands::pf_delete_rule,
            portforward::commands::pf_list_rules,
            portforward::commands::pf_start_tunnel,
            portforward::commands::pf_stop_tunnel,
            portforward::commands::pf_list_active_tunnels,
            // Snippets
            snippets::commands::save_snippet,
            snippets::commands::get_snippet,
            snippets::commands::list_snippets,
            snippets::commands::delete_snippet,
            snippets::commands::search_snippets,
            snippets::commands::record_snippet_use,
            snippets::commands::save_snippet_folder,
            snippets::commands::list_snippet_folders,
            snippets::commands::delete_snippet_folder,
            snippets::commands::snippet_execute,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn migrate_legacy_vault_entries(host_db: &HostDb) {
    let host_keys = host_db
        .list_hosts()
        .map(|hosts| hosts.into_iter().map(|host| host.id).collect::<Vec<_>>());
    let s3_keys = host_db.list_s3_connections().map(|connections| {
        connections
            .into_iter()
            .flat_map(|connection| {
                [
                    format!("s3:{}", connection.id),
                    format!("r2-admin:{}", connection.id),
                ]
            })
            .collect::<Vec<_>>()
    });

    let mut vault_keys = Vec::new();
    match host_keys {
        Ok(keys) => vault_keys.extend(keys),
        Err(err) => {
            tracing::warn!(error = %err, "could not inspect hosts for legacy keychain migration")
        }
    }
    match s3_keys {
        Ok(keys) => vault_keys.extend(keys),
        Err(err) => {
            tracing::warn!(error = %err, "could not inspect S3 connections for legacy keychain migration")
        }
    }

    match vault::migrate_legacy_credentials(&vault_keys) {
        Ok(count) if count > 0 => tracing::info!(
            credential_count = count,
            "migrated legacy keychain entries to Retoom"
        ),
        Ok(_) => {}
        Err(err) => tracing::warn!(error = %err, "legacy keychain migration failed"),
    }
}
