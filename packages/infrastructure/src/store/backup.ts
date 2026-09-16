import fs from "node:fs";
import path from "node:path";
import Database from "better-sqlite3";

/**
 * SQLite database backup utility.
 * Uses the SQLite Online Backup API (via better-sqlite3).
 */
export class DatabaseBackup {
  /**
   * Create a backup of the database.
   * File name: poria-YYYYMMDD.db
   */
  async backup(dbPath: string, backupDir: string): Promise<string> {
    fs.mkdirSync(backupDir, { recursive: true });

    const date = new Date().toISOString().slice(0, 10).replace(/-/g, "");
    const backupFileName = `poria-${date}.db`;
    const backupPath = path.join(backupDir, backupFileName);

    // Use better-sqlite3's backup API (non-blocking via async backup)
    const db = new Database(dbPath, { readonly: true });
    try {
      await db.backup(backupPath);
    } finally {
      db.close();
    }

    return backupPath;
  }

  /**
   * Remove backups older than retainDays.
   */
  cleanup(backupDir: string, retainDays = 7): number {
    if (!fs.existsSync(backupDir)) return 0;

    const cutoffMs = Date.now() - retainDays * 86400_000;
    const files = fs.readdirSync(backupDir);
    let removed = 0;

    for (const file of files) {
      if (!file.startsWith("poria-") || !file.endsWith(".db")) continue;

      // Extract date from filename: poria-YYYYMMDD.db
      const dateStr = file.slice(6, 14); // "YYYYMMDD"
      const year = parseInt(dateStr.slice(0, 4), 10);
      const month = parseInt(dateStr.slice(4, 6), 10) - 1;
      const day = parseInt(dateStr.slice(6, 8), 10);

      if (isNaN(year) || isNaN(month) || isNaN(day)) continue;

      const fileDate = new Date(year, month, day);
      if (fileDate.getTime() < cutoffMs) {
        const filePath = path.join(backupDir, file);
        fs.unlinkSync(filePath);
        removed++;
      }
    }

    return removed;
  }
}
