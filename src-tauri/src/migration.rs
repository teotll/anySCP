use std::path::{Path, PathBuf};

const CURRENT_DB: &str = "retoom.db";
const LEGACY_DB: &str = "anyscp.db";
// Retoom is macOS-only by design; these are Tauri app-data directory names
// under ~/Library/Application Support from the original anySCP bundle/name.
const LEGACY_APP_DIRS: &[&str] = &["com.macnev2013.anyscp", "anySCP", "anyscp"];

pub fn migrate_legacy_app_state(app_data_dir: &Path) -> Result<(), String> {
    copy_legacy_database_if_needed(app_data_dir)
}

fn copy_legacy_database_if_needed(app_data_dir: &Path) -> Result<(), String> {
    let current_db = app_data_dir.join(CURRENT_DB);
    if current_db.exists() {
        return Ok(());
    }

    let Some(legacy_db) = find_legacy_database(app_data_dir) else {
        return Ok(());
    };

    std::fs::create_dir_all(app_data_dir).map_err(|err| {
        format!(
            "could not create Retoom app data directory {}: {err}",
            app_data_dir.display()
        )
    })?;

    copy_database_file(&legacy_db, &current_db)?;
    for suffix in ["-wal", "-shm"] {
        let legacy_sidecar = PathBuf::from(format!("{}{suffix}", legacy_db.display()));
        if legacy_sidecar.exists() {
            let current_sidecar = PathBuf::from(format!("{}{suffix}", current_db.display()));
            copy_database_file(&legacy_sidecar, &current_sidecar)?;
        }
    }

    tracing::info!(
        from = %legacy_db.display(),
        to = %current_db.display(),
        "migrated legacy anySCP database to Retoom"
    );
    Ok(())
}

fn find_legacy_database(app_data_dir: &Path) -> Option<PathBuf> {
    legacy_app_data_dirs(app_data_dir)
        .into_iter()
        .map(|dir| dir.join(LEGACY_DB))
        .find(|db| db.exists())
}

fn legacy_app_data_dirs(app_data_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(parent) = app_data_dir.parent() {
        for legacy_dir in LEGACY_APP_DIRS {
            dirs.push(parent.join(legacy_dir));
        }
    }
    dirs.push(app_data_dir.to_path_buf());
    dedupe_paths(dirs)
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut deduped = Vec::new();
    for path in paths {
        if !deduped.iter().any(|existing| existing == &path) {
            deduped.push(path);
        }
    }
    deduped
}

fn copy_database_file(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::copy(from, to).map(|_| ()).map_err(|err| {
        format!(
            "could not copy legacy database file {} to {}: {err}",
            from.display(),
            to.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_legacy_database_when_retoom_database_is_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let new_dir = temp.path().join("com.teotll.retoom");
        let old_dir = temp.path().join("com.macnev2013.anyscp");
        std::fs::create_dir_all(&old_dir).expect("old dir");
        std::fs::write(old_dir.join(LEGACY_DB), "legacy-db").expect("old db");
        std::fs::write(old_dir.join("anyscp.db-wal"), "legacy-wal").expect("old wal");

        migrate_legacy_app_state(&new_dir).expect("migrate");

        assert_eq!(
            std::fs::read_to_string(new_dir.join(CURRENT_DB)).expect("new db"),
            "legacy-db"
        );
        assert_eq!(
            std::fs::read_to_string(new_dir.join("retoom.db-wal")).expect("new wal"),
            "legacy-wal"
        );
    }

    #[test]
    fn does_not_overwrite_existing_retoom_database() {
        let temp = tempfile::tempdir().expect("tempdir");
        let new_dir = temp.path().join("com.teotll.retoom");
        let old_dir = temp.path().join("com.macnev2013.anyscp");
        std::fs::create_dir_all(&new_dir).expect("new dir");
        std::fs::create_dir_all(&old_dir).expect("old dir");
        std::fs::write(new_dir.join(CURRENT_DB), "current-db").expect("new db");
        std::fs::write(old_dir.join(LEGACY_DB), "legacy-db").expect("old db");

        migrate_legacy_app_state(&new_dir).expect("migrate");

        assert_eq!(
            std::fs::read_to_string(new_dir.join(CURRENT_DB)).expect("new db"),
            "current-db"
        );
    }
}
