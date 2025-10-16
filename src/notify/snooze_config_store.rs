use std::fs;
use std::path::{Path, PathBuf};
use glib::DateTime;
use rusqlite::{params, Connection, OptionalExtension};
use crate::{CONFIG_DIR_NAME, DB_FILE_NAME};

#[derive(Clone, Debug)]
pub struct SnoozeConfigStore {
    path: PathBuf,
}

impl SnoozeConfigStore {

    /// Open (and create/migrate) the config DB under ~/.config/github-notifier/config.db
    pub fn open_default() -> rusqlite::Result<Self> {
        let dir = dirs::home_dir()
            .expect("no home dir")
            .join(CONFIG_DIR_NAME);
        fs::create_dir_all(&dir).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        let path = dir.join(DB_FILE_NAME);
        Self::open_at(path)
    }

    /// Open at a specific path (useful for tests)
    pub fn open_at<P: AsRef<Path>>(path: P) -> rusqlite::Result<Self> {
        let conn = Connection::open(path.as_ref())?;
        Self::migrate(&conn)?;
        Ok(Self { path: path.as_ref().to_path_buf() })
    }

    fn connect(&self) -> rusqlite::Result<Connection> {
        let conn = Connection::open(&self.path)?;
        // Reasonable pragmas for tiny config DBs
        conn.pragma_update(None, "journal_mode", &"WAL")?;
        conn.pragma_update(None, "synchronous", &"NORMAL")?;
        Ok(conn)
    }

    fn migrate(conn: &Connection) -> rusqlite::Result<()> {
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS snoozed_repositories(
              owner   TEXT NOT NULL,
              repo    TEXT NOT NULL,
              until   INTEGER,
              reason  TEXT,
              created_at INTEGER NOT NULL DEFAULT (unixepoch('now')),
              updated_at INTEGER NOT NULL DEFAULT (unixepoch('now')),
              UNIQUE(owner, repo)
            );

            CREATE TABLE IF NOT EXISTS snoozed_repository_reasons(
              owner TEXT NOT NULL,
              repo  TEXT NOT NULL,
              reason TEXT NOT NULL,
              UNIQUE(owner, repo, reason)
            );
            "#,
        )?;
        Ok(())
    }

    /// Remove expired rows (safe to call often)
    pub fn prune_expired(&self, now: DateTime) -> rusqlite::Result<usize> {
        let conn = self.connect()?;
        let now_sec = now.to_unix();

        let mut total = 0;
        total += conn.execute("DELETE FROM snoozed_repositories WHERE until <= ?", params![now_sec])?;
        Ok(total)
    }

    /// Snooze a repo until a given UTC instant (overwrites existing)
    pub fn snooze_repo(&self, owner: &str, repo: &str, until: DateTime) -> rusqlite::Result<()> {
        let conn = self.connect()?;
        let until_sec = until.to_unix();
        conn.execute(
            r#"
            INSERT INTO snoozed_repositories(owner, repo, until)
            VALUES(?, ?, ?)
            ON CONFLICT(owner, repo) DO UPDATE SET until=excluded.until
            "#,
            params![owner, repo, until_sec],
        )?;
        Ok(())
    }

    /// Adds repository to table
    pub fn add_repo(&self, owner: &str, repo: &str) -> rusqlite::Result<()> {
        let conn = self.connect()?;

        conn.execute(
            r#"
            INSERT INTO snoozed_repositories(owner, repo)
            VALUES(?, ?)
            ON CONFLICT(owner, repo) DO NOTHING
            "#,
            params![owner, repo],
        )?;
        Ok(())
    }

    /// Unsnooze a repo
    pub fn unsnooze_repo(&self, owner: &str, repo: &str) -> rusqlite::Result<bool> {
        let conn = self.connect()?;
        let changed = conn.execute("DELETE FROM snoozed_repositories WHERE owner=? AND repo=?", params![owner, repo])?;
        Ok(changed > 0)
    }

    /// Check repo snoozed
    pub fn is_repo_snoozed(&self, owner: &str, repo: &str, now: DateTime) -> rusqlite::Result<bool> {
        let conn = self.connect()?;
        let now_sec = now.to_unix();
        let until_opt: Option<i64> = conn.query_row(
            "SELECT until FROM snoozed_repositories WHERE owner=? AND repo=?",
            params![owner, repo],
            |row| row.get(0),
        ).optional()?;
        Ok(matches!(until_opt, Some(until) if until > now_sec))
    }


    pub fn list_snoozed_repos(&self, now: DateTime) -> rusqlite::Result<Vec<(String, String, DateTime)>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT owner, repo, until FROM snoozed_repositories WHERE until > ? ORDER BY until DESC")?;
        let rows = stmt.query_map(params![now.seconds()], |row| {
            let until: i64 = row.get(2)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                DateTime::from_unix_utc(until).unwrap(),
            ))
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }
    pub fn list_all_repos(&self) -> rusqlite::Result<Vec<(String, String)>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT owner, repo FROM snoozed_repositories ORDER BY repo ASC")?;
        let rows = stmt.query_map(params![], |row| {

            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?
            ))
        })?;
        Ok(rows.filter_map(Result::ok).collect())
    }

    pub fn snooze_reason(
        &self,
        owner: &str,
        repo: &str,
        reason: Option<&str>,
    ) -> rusqlite::Result<()> {
        let conn = self.connect()?;

        let mut insert_stmt = conn.execute(
            "INSERT OR IGNORE INTO snoozed_repository_reasons(owner, repo, reason) VALUES(?, ?, ?)",
            params![owner, repo, reason.unwrap_or("")],
        )?;


        Ok(())
    }
    pub fn unsooze_reason(
        &self,
        owner: &str,
        repo: &str,
        reason: Option<&str>,
    ) -> rusqlite::Result<()> {
        let conn = self.connect()?;

        let mut insert_stmt = conn.execute(
            "DELETE FROM snoozed_repository_reasons WHERE owner=? AND repo=? AND reason=?",
            params![owner, repo, reason.unwrap_or("")],
        )?;


        Ok(())
    }

    /// Toggle a single reason. Returns the NEW state (true = now snoozed).
    pub fn toggle_reason(
        &self,
        owner: &str,
        repo: &str,
        reason: &str,
    ) -> rusqlite::Result<bool> {
        let conn = self.connect()?;
        let tx = conn.unchecked_transaction()?;

        let exists: Option<i64> = tx.query_row(
            "SELECT 1 FROM snoozed_repository_reasons
             WHERE owner=? AND repo=? AND reason=? LIMIT 1",
            params![owner, repo, reason],
            |row| row.get(0),
        ).optional()?;

        let now_enabled = if exists.is_some() {
            tx.execute(
                "DELETE FROM snoozed_repository_reasons
                 WHERE owner=? AND repo=? AND reason=?",
                params![owner, repo, reason],
            )?;
            false
        } else {
            tx.execute(
                "INSERT OR IGNORE INTO snoozed_repository_reasons(owner, repo, reason)
                 VALUES(?, ?, ?)",
                params![owner, repo, reason],
            )?;
            true
        };

        tx.commit()?;
        Ok(now_enabled)
    }

    pub fn is_repo_snoozed_for_reason(
        &self,
        owner: &str,
        repo: &str,
        reason: &str,
    ) -> rusqlite::Result<bool> {
        let conn = self.connect()?;


        let any: Option<i64> = conn
            .query_row(
        r#"
            SELECT 1
              FROM snoozed_repository_reasons
             WHERE owner = ?1 AND repo = ?2 AND reason = ?3
             LIMIT 1
            "#,
                params![owner, repo, reason],
                |row| row.get(0),
            )
            .optional()?;

        Ok(any.is_some())
    }
    pub fn should_snooze_for_reason(
        &self,
        owner: &str,
        repo: &str,
        reason: &str,
        now: DateTime,
    ) -> rusqlite::Result<bool> {
        let conn = self.connect()?;
        let now_sec = now.to_unix();

        let any: Option<i64> = conn
            .query_row(
                r#"
            SELECT 1
              FROM (
                    SELECT 1
                      FROM snoozed_repositories
                     WHERE owner = ?1 AND repo = ?2 AND until > ?3
                    UNION ALL
                    SELECT 1
                      FROM snoozed_repository_reasons
                     WHERE owner = ?1 AND repo = ?2 AND reason = ?4
                   )
             LIMIT 1
            "#,
                params![owner, repo, now_sec, reason],
                |row| row.get(0),
            )
            .optional()?;

        Ok(any.is_some())
    }
}