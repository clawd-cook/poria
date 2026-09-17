use std::path::Path;

pub struct DatabaseBackup;

impl DatabaseBackup {
    pub fn backup(db_path: &Path, backup_dir: &Path) -> Result<String, String> {
        std::fs::create_dir_all(backup_dir).map_err(|e| e.to_string())?;

        let date = chrono::Utc::now().format("%Y%m%d").to_string();
        let backup_name = format!("poria-{}.db", date);
        let backup_path = backup_dir.join(&backup_name);

        std::fs::copy(db_path, &backup_path).map_err(|e| e.to_string())?;

        Ok(backup_path.to_string_lossy().to_string())
    }

    pub fn cleanup(backup_dir: &Path, retain_days: i64) -> Result<usize, String> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(retain_days);
        let cutoff_str = cutoff.format("%Y%m%d").to_string();
        let mut removed = 0;

        let entries = std::fs::read_dir(backup_dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(date_str) = name
                .strip_prefix("poria-")
                .and_then(|s| s.strip_suffix(".db"))
            {
                if date_str < cutoff_str.as_str() {
                    std::fs::remove_file(entry.path()).ok();
                    removed += 1;
                }
            }
        }

        Ok(removed)
    }
}
