use crate::error::{Error, Result};
use crate::types::*;
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params, params_from_iter};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Store {
    conn: Mutex<Connection>,
    /// A separate WAL reader keeps list/search queries off the writer lock.
    reader: Option<Mutex<Connection>>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        let store = Self::init(Connection::open(path)?)?;
        let reader = Connection::open(path)?;
        reader.busy_timeout(std::time::Duration::from_millis(5_000))?;
        reader.pragma_update(None, "query_only", true)?;
        Ok(Store { reader: Some(Mutex::new(reader)), ..store })
    }

    /// In-memory store for tests and short-lived tooling.
    pub fn open_in_memory() -> Result<Store> {
        let conn = Connection::open_in_memory()?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Store> {
        conn.busy_timeout(std::time::Duration::from_millis(5_000))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let store = Store { conn: Mutex::new(conn), reader: None };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.lock();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version < 1 {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(SCHEMA_V1)?;
            tx.pragma_update(None, "user_version", 1)?;
            tx.commit()?;
        }
        if version < 2 {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(
                "ALTER TABLE files ADD COLUMN scan_gen INTEGER NOT NULL DEFAULT 0;
                 CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT OR IGNORE INTO meta(key, value) VALUES('scan_gen', '0');",
            )?;
            tx.pragma_update(None, "user_version", 2)?;
            tx.commit()?;
        }
        if version < 3 {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch("ALTER TABLE files ADD COLUMN duration_ms INTEGER;")?;
            tx.pragma_update(None, "user_version", 3)?;
            tx.commit()?;
        }
        if version < 4 {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch("UPDATE files SET added_at = mtime WHERE added_at > mtime;")?;
            tx.pragma_update(None, "user_version", 4)?;
            tx.commit()?;
        }
        if version < 5 {
            let tx = conn.unchecked_transaction()?;
            // Release archives contain platform executables (node, electron,
            // helpers) that are not applications. A rescan applies the same
            // bundle-launcher rule to future rows.
            tx.execute_batch(
                "UPDATE files SET kind = 'other'
                 WHERE kind = 'app' AND ext = '' AND path LIKE '%/releases/%';",
            )?;
            tx.pragma_update(None, "user_version", 5)?;
            tx.commit()?;
        }
        Ok(())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn read(&self) -> std::sync::MutexGuard<'_, Connection> {
        match &self.reader {
            Some(reader) => reader.lock().unwrap_or_else(std::sync::PoisonError::into_inner),
            None => self.lock(),
        }
    }

    // ---------------------------------------------------------------- scanning

    /// Insert-or-update a batch of scanned files in one transaction. Returns the number of rows.
    pub fn upsert_batch(&self, files: &[ScannedFile]) -> Result<usize> {
        self.upsert_batch_at(files, None)
    }

    /// Upsert while marking rows as seen by scan generation `gen`.
    pub fn upsert_batch_at(&self, files: &[ScannedFile], generation: Option<i64>) -> Result<usize> {
        if files.is_empty() {
            return Ok(0);
        }
        let mut conn = self.lock();
        let tx = conn.transaction()?;
        let mut changed = 0;
        for f in files {
            changed += upsert_one(&tx, f, generation)?;
        }
        tx.commit()?;
        Ok(changed)
    }

    pub fn upsert_file(&self, f: &ScannedFile) -> Result<bool> {
        self.upsert_file_at(f, None)
    }

    pub fn upsert_file_at(&self, f: &ScannedFile, generation: Option<i64>) -> Result<bool> {
        let mut conn = self.lock();
        let tx = conn.transaction()?;
        let changed = upsert_one(&tx, f, generation)?;
        tx.commit()?;
        Ok(changed > 0)
    }

    /// Start a full scan generation. Files seen in this scan get this value.
    pub fn begin_scan(&self) -> Result<i64> {
        let conn = self.lock();
        // One statement, so two processes cannot interleave bump and read-back
        // and then prune each other's freshly stamped rows in finish_scan.
        Ok(conn.query_row(
            "INSERT INTO meta(key, value) VALUES('scan_gen', '1')
             ON CONFLICT(key) DO UPDATE SET value = CAST(value AS INTEGER) + 1
             RETURNING CAST(value AS INTEGER)",
            [],
            |r| r.get(0),
        )?)
    }

    pub fn meta_get(&self, key: &str) -> Result<Option<String>> {
        let conn = self.read();
        Ok(conn
            .query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| r.get(0))
            .optional()?)
    }

    pub fn meta_set(&self, key: &str, value: &str) -> Result<()> {
        self.lock().execute(
            "INSERT INTO meta(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn duration_for(&self, path: &Path) -> Result<Option<u64>> {
        let conn = self.read();
        let value: Option<Option<i64>> = conn
            .query_row(
                "SELECT duration_ms FROM files WHERE path = ?1",
                params![path_str(path)],
                |r| r.get(0),
            )
            .optional()?;
        Ok(value.flatten().map(|v| v.max(0) as u64))
    }

    pub fn current_scan_gen(&self) -> Result<i64> {
        let conn = self.read();
        Ok(conn
            .query_row("SELECT CAST(value AS INTEGER) FROM meta WHERE key = 'scan_gen'", [], |r| {
                r.get(0)
            })
            .unwrap_or(0))
    }

    /// Delete rows under `root` that no full scan has seen. `started` protects
    /// files created while the walk was running (their mtime is newer).
    pub fn finish_scan(&self, root: &Path, generation: i64, started: i64) -> Result<usize> {
        self.finish_scan_excluding(root, generation, started, &[])
    }

    /// Prune unseen rows while keeping every path under an unreadable subtree.
    /// One permission-denied directory must not freeze stale rows for the whole
    /// root, but deleting rows we could not inspect would be worse.
    pub fn finish_scan_excluding(
        &self,
        root: &Path,
        generation: i64,
        started: i64,
        protected: &[PathBuf],
    ) -> Result<usize> {
        let root_s = path_str(root);
        let like = format!("{}/%", escape_like(&root_s));
        let mut sql = String::from(
            "DELETE FROM files
             WHERE in_trash = 0 AND scan_gen <> ?1
               AND (scan_gen <> 0 OR mtime < ?2)
               AND (path = ?3 OR path LIKE ?4 ESCAPE '\\')",
        );
        let mut args: Vec<Box<dyn rusqlite::types::ToSql>> =
            vec![Box::new(generation), Box::new(started), Box::new(root_s), Box::new(like)];
        for prefix in protected {
            let escaped = escape_like(&path_str(prefix));
            sql.push_str(&format!(
                " AND path <> ?{} AND path NOT LIKE ?{} ESCAPE '\\'",
                args.len() + 1,
                args.len() + 2
            ));
            args.push(Box::new(path_str(prefix)));
            args.push(Box::new(format!("{escaped}/%")));
        }
        let conn = self.lock();
        let refs: Vec<&dyn rusqlite::types::ToSql> = args.iter().map(|a| a.as_ref()).collect();
        Ok(conn.execute(&sql, refs.as_slice())?)
    }

    /// Remove index rows for paths that vanished since the last scan.
    pub fn delete_paths(&self, paths: &[PathBuf]) -> Result<usize> {
        let conn = self.lock();
        let mut n = 0;
        for p in paths {
            n += conn.execute("DELETE FROM files WHERE path = ?1", params![path_str(p)])?;
        }
        Ok(n)
    }

    /// Move a file or folder and rewrite the indexed paths beneath it.
    pub fn rename_path(&self, old: &Path, new: &Path, kind: FileKind) -> Result<()> {
        let old_s = path_str(old);
        let new_s = path_str(new);
        let parent = new.parent().map(path_str).unwrap_or_default();
        let name = new.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let ext = new.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let kind = kind.as_str().to_string();
        let like = format!("{}/%", escape_like(&old_s));
        let mut conn = self.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "UPDATE files SET path = ?2, parent = ?3, name = ?4, ext = ?5, kind = ?6 WHERE path = ?1",
            params![old_s, new_s, parent, name, ext, kind],
        )?;
        let prefix_len = old_s.chars().count() as i64 + 1;
        tx.execute(
            "UPDATE files
             SET path = ?2 || substr(path, ?3),
                 parent = ?2 || substr(parent, ?3)
             WHERE path LIKE ?4 ESCAPE '\\'",
            params![old_s, new_s, prefix_len, like],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Drop all rows under a prefix (used when a watched root is unindexed).
    pub fn prune_under_path(&self, prefix: &Path) -> Result<usize> {
        let like = format!("{}/%", escape_like(&path_str(prefix)));
        let conn = self.lock();
        conn.execute(
            "DELETE FROM files WHERE path = ?1 OR path LIKE ?2 ESCAPE '\\'",
            params![path_str(prefix), like],
        )
        .map_err(Into::into)
    }

    // ------------------------------------------------------------------ lookup

    pub fn by_path(&self, path: &Path) -> Result<Option<FileMeta>> {
        let conn = self.read();
        let row = conn
            .query_row(&base_select(" WHERE f.path = ?1"), params![path_str(path)], map_meta)
            .optional()?;
        Ok(row)
    }

    pub fn get_files(&self, ids: &[i64]) -> Result<Vec<FileMeta>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("{} WHERE f.id IN ({})", base_select(""), placeholders);
        let conn = self.read();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(ids.iter()), map_meta)?;
        let mut out = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        out.sort_by_key(|f| ids.iter().position(|id| id == &f.id).unwrap_or(usize::MAX));
        Ok(out)
    }

    /// One page of the list. `cursor` comes from the previous page's `next_cursor`.
    pub fn list(&self, q: &Query) -> Result<Page> {
        let limit = q.limit.clamp(1, 500);
        let mut where_parts: Vec<String> = vec![];
        let mut args: Vec<Box<dyn rusqlite::types::ToSql>> = vec![];

        where_parts.push("f.in_trash = ?".into());
        args.push(Box::new(q.in_trash as i64));

        if !q.kinds.is_empty() {
            let placeholders = q.kinds.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            where_parts.push(format!("f.kind IN ({placeholders})"));
            for k in &q.kinds {
                args.push(Box::new(k.as_str().to_string()));
            }
        }
        if !q.exts.is_empty() {
            let placeholders = q.exts.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            where_parts.push(format!("f.ext IN ({placeholders})"));
            for e in &q.exts {
                args.push(Box::new(e.to_lowercase()));
            }
        }
        if let Some(ss) = q.screenshot {
            where_parts.push("f.is_screenshot = ?".into());
            args.push(Box::new(ss as i64));
        }
        if let Some(v) = q.mtime_from {
            where_parts.push("f.mtime >= ?".into());
            args.push(Box::new(v));
        }
        if let Some(v) = q.mtime_to {
            where_parts.push("f.mtime <= ?".into());
            args.push(Box::new(v));
        }
        if let Some(v) = q.min_size {
            where_parts.push("f.size >= ?".into());
            args.push(Box::new(v));
        }
        if let Some(v) = q.max_size {
            where_parts.push("f.size <= ?".into());
            args.push(Box::new(v));
        }
        if !q.sources.is_empty() {
            let parts = q
                .sources
                .iter()
                .map(|_| {
                    "(s.url LIKE ? ESCAPE '\\' OR s.app LIKE ? ESCAPE '\\' OR s.host LIKE ? ESCAPE '\\')"
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(" OR ");
            where_parts.push(format!("({parts})"));
            for src in &q.sources {
                let like = format!("%{}%", escape_like(src));
                args.push(Box::new(like.clone()));
                args.push(Box::new(like.clone()));
                args.push(Box::new(like));
            }
        }
        if q.exclude_code {
            where_parts.push(
                "f.ext NOT IN ('rs','ts','tsx','js','jsx','mjs','cjs','py','go','java','kt','kts','c','h','cc','cpp','hpp','cs','rb','php','swift','scala','lua','sh','bash','zsh','fish','sql','html','htm','css','scss','sass','less','vue','svelte','astro','json','jsonc','toml','yaml','yml','xml','gradle','cmake','make','mk','proto','graphql','gql','ex','exs','erl','hrl','clj','cljs','dart','r','jl','nim','zig','v','sol','tf','hcl')"
                    .into(),
            );
        }
        if let Some(prefix) = &q.under_path {
            if q.direct_children {
                where_parts.push("f.parent = ?".into());
                args.push(Box::new(prefix.clone()));
            } else {
                where_parts.push("(f.path = ? OR f.path LIKE ? ESCAPE '\\')".into());
                args.push(Box::new(prefix.clone()));
                args.push(Box::new(format!("{}/%", escape_like(prefix))));
            }
        }
        for name in q.names.iter() {
            where_parts.push(format!("f.name = ?{}", args.len() + 1));
            args.push(Box::new(name.clone()));
        }
        for tag in q.tags.iter() {
            where_parts.push(format!(
                "f.id IN (SELECT file_id FROM tags WHERE tag = ?{})",
                args.len() + 1
            ));
            args.push(Box::new(tag.clone()));
        }
        for term in q.text.iter() {
            if term.chars().count() >= 3 {
                where_parts.push(format!(
                    "f.id IN (SELECT rowid FROM files_fts WHERE files_fts MATCH ?{})",
                    args.len() + 1
                ));
                args.push(Box::new(fts_phrase(term)));
            } else {
                where_parts.push(format!("f.name LIKE ?{} ESCAPE '\\'", args.len() + 1));
                args.push(Box::new(format!("%{}%", escape_like(term))));
            }
        }

        let sort = SortKey::of(q.sort);
        if let Some(cursor) = &q.cursor {
            let (key, id) = decode_cursor(cursor)?;
            where_parts.push(match sort.direction {
                Dir::Desc => "({col} < ? OR ({col} = ? AND f.id < ?))".replace("{col}", sort.col),
                Dir::Asc => "({col} > ? OR ({col} = ? AND f.id > ?))".replace("{col}", sort.col),
            });
            args.push(Box::new(key.clone()));
            args.push(Box::new(key));
            args.push(Box::new(id));
        }

        let sql = format!(
            "{} WHERE {} ORDER BY {} {}, f.id {} LIMIT {}",
            base_select(""),
            where_parts.join(" AND "),
            sort.col,
            match sort.direction {
                Dir::Desc => "DESC",
                Dir::Asc => "ASC",
            },
            match sort.direction {
                Dir::Desc => "DESC",
                Dir::Asc => "ASC",
            },
            limit + 1,
        );

        let conn = self.read();
        let mut stmt = conn.prepare(&sql)?;
        let refs: Vec<&dyn rusqlite::types::ToSql> = args.iter().map(|a| a.as_ref()).collect();
        let mut rows = stmt.query_map(refs.as_slice(), map_meta)?;
        let items: Vec<FileMeta> =
            rows.by_ref().take(limit as usize).collect::<std::result::Result<_, _>>()?;
        // Count what is left by hand: rows.count() would turn a row-mapping
        // error into a phantom extra row and a cursor for a page that cannot load.
        let mut has_more = false;
        for row in rows.by_ref() {
            row?;
            has_more = true;
        }

        let next_cursor = match items.last() {
            Some(last) if has_more => {
                let key: Value = sort.key_of(last);
                Some(encode_cursor(&key, last.id)?)
            }
            _ => None,
        };
        Ok(Page { items, next_cursor })
    }

    // ------------------------------------------------------------------- stats

    pub fn kind_stats(&self) -> Result<Vec<KindStat>> {
        let conn = self.read();
        let mut stmt = conn.prepare(
            "SELECT kind, COUNT(*), COALESCE(SUM(size), 0) FROM files
             WHERE in_trash = 0 GROUP BY kind",
        )?;
        let rows = stmt.query_map([], |r| {
            let kind: String = r.get(0)?;
            Ok(KindStat {
                kind: FileKind::ALL
                    .into_iter()
                    .find(|k| k.as_str() == kind)
                    .unwrap_or(FileKind::Other),
                count: r.get::<_, i64>(1)? as u64,
                bytes: r.get::<_, i64>(2)? as u64,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn totals(&self) -> Result<(u64, u64)> {
        let conn = self.read();
        conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(size), 0) FROM files WHERE in_trash = 0",
            [],
            |r| Ok((r.get::<_, i64>(0)? as u64, r.get::<_, i64>(1)? as u64)),
        )
        .map_err(Into::into)
    }

    /// Count and bytes of everything currently in the Trash view.
    pub fn trash_totals(&self) -> Result<(u64, u64)> {
        let conn = self.read();
        conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(size), 0) FROM files WHERE in_trash = 1",
            [],
            |r| Ok((r.get::<_, i64>(0)? as u64, r.get::<_, i64>(1)? as u64)),
        )
        .map_err(Into::into)
    }

    pub fn skills(&self, limit: u32) -> Result<Vec<FileMeta>> {
        let q = Query {
            names: vec!["SKILL.md".into()],
            sort: Sort::Modified,
            limit,
            ..Query::default()
        };
        Ok(self.list(&q)?.items)
    }

    pub fn skill_stats(&self) -> Result<(u64, u64)> {
        let conn = self.read();
        Ok(conn.query_row(
            "SELECT COUNT(*), COALESCE(SUM(size), 0) FROM files
             WHERE in_trash = 0 AND lower(name) = 'skill.md'",
            [],
            |r| Ok((r.get::<_, i64>(0)? as u64, r.get::<_, i64>(1)? as u64)),
        )?)
    }

    /// Files whose size collides with another live file. Duplicate detection
    /// hashes only these candidates rather than the whole index.
    pub fn duplicate_candidates(&self) -> Result<Vec<FileMeta>> {
        let sql = format!(
            "{} WHERE f.in_trash = 0 AND f.size > 0 AND f.size IN (
               SELECT size FROM files WHERE in_trash = 0 AND size > 0
               GROUP BY size HAVING COUNT(*) > 1
             ) ORDER BY f.size DESC",
            base_select("")
        );
        let conn = self.read();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], map_meta)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn recent(&self, limit: u32) -> Result<Vec<FileMeta>> {
        let q = Query { sort: Sort::Added, limit, exclude_code: true, ..Query::default() };
        Ok(self.list(&q)?.items)
    }

    // -------------------------------------------------------------- sources

    pub fn set_source(
        &self,
        path: &Path,
        url: &str,
        app: Option<&str>,
        host: Option<&str>,
    ) -> Result<()> {
        if url.is_empty() {
            return Ok(());
        }
        let mut conn = self.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO sources(url, app, host) VALUES (?1, ?2, ?3)
             ON CONFLICT(url) DO UPDATE SET app = COALESCE(?2, app), host = COALESCE(?3, host)",
            params![url, app, host],
        )?;
        tx.execute(
            "UPDATE files SET source_id = (SELECT id FROM sources WHERE url = ?2) WHERE path = ?1",
            params![path_str(path), url],
        )?;
        tx.commit()?;
        Ok(())
    }

    // ----------------------------------------------------------------- tags

    pub fn set_tags(&self, path: &Path, tags: &[String]) -> Result<()> {
        let mut conn = self.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM tags WHERE file_id = (SELECT id FROM files WHERE path = ?1)",
            params![path_str(path)],
        )?;
        for tag in tags {
            tx.execute(
                "INSERT OR IGNORE INTO tags(file_id, tag)
                 VALUES ((SELECT id FROM files WHERE path = ?1), ?2)",
                params![path_str(path), tag],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn tags_of(&self, path: &Path) -> Result<Vec<String>> {
        let conn = self.read();
        let mut stmt = conn.prepare(
            "SELECT t.tag FROM tags t JOIN files f ON f.id = t.file_id WHERE f.path = ?1 ORDER BY t.tag",
        )?;
        let rows = stmt.query_map(params![path_str(path)], |r| r.get(0))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    // ---------------------------------------------------------- saved searches

    pub fn save_search(&self, name: &str, query: &str) -> Result<i64> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO saved_searches(name, query, created) VALUES (?1, ?2, strftime('%s','now'))
             ON CONFLICT(name) DO UPDATE SET query = excluded.query",
            params![name, query],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_saved_searches(&self) -> Result<Vec<SavedSearch>> {
        let conn = self.read();
        let mut stmt =
            conn.prepare("SELECT id, name, query, created FROM saved_searches ORDER BY created")?;
        let rows = stmt.query_map([], |r| {
            Ok(SavedSearch { id: r.get(0)?, name: r.get(1)?, query: r.get(2)?, created: r.get(3)? })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn delete_saved_search(&self, id: i64) -> Result<()> {
        let conn = self.lock();
        conn.execute("DELETE FROM saved_searches WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ------------------------------------------------------- trash + receipts

    /// Record that files were moved to Trash by the platform layer.
    pub fn mark_trashed(&self, moved: &[(PathBuf, PathBuf)], detail: Option<&str>) -> Result<i64> {
        if moved.is_empty() {
            return Ok(0);
        }
        let entries: Vec<_> = moved
            .iter()
            .map(|(orig, trashed)| serde_json::json!({ "path": path_str(orig), "trash_path": path_str(trashed) }))
            .collect();
        let mut conn = self.lock();
        let tx = conn.transaction()?;
        for (orig, trashed) in moved {
            tx.execute(
                "UPDATE files SET in_trash = 1, trash_path = ?2 WHERE path = ?1",
                params![path_str(orig), path_str(trashed)],
            )?;
        }
        tx.execute(
            "INSERT INTO receipts(action, paths, detail, created) VALUES ('trash', ?1, ?2, strftime('%s','now'))",
            params![serde_json::to_string(&entries)?, detail],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(id)
    }

    pub fn receipt_paths(&self, receipt_id: i64) -> Result<Vec<(String, String)>> {
        let conn = self.read();
        let paths: String = conn
            .query_row("SELECT paths FROM receipts WHERE id = ?1", params![receipt_id], |r| {
                r.get(0)
            })
            .optional()?
            .ok_or(Error::InvalidCursor)?;
        let entries: Vec<Value> = serde_json::from_str(&paths)?;
        Ok(entries
            .into_iter()
            .filter_map(|e| {
                Some((
                    e.get("path")?.as_str()?.to_string(),
                    e.get("trash_path")?.as_str()?.to_string(),
                ))
            })
            .collect())
    }

    /// Mark one receipt's entries as restored after the platform layer moved files back.
    pub fn mark_restored(&self, receipt_id: i64, trash_paths: &[PathBuf]) -> Result<usize> {
        let conn = self.lock();
        let mut n = 0;
        for tp in trash_paths {
            n += conn.execute(
                "UPDATE files SET in_trash = 0, trash_path = NULL WHERE trash_path = ?1",
                params![path_str(tp)],
            )?;
        }
        conn.execute("UPDATE receipts SET undone = 1 WHERE id = ?1", params![receipt_id])?;
        Ok(n)
    }

    pub fn list_receipts(&self) -> Result<Vec<Receipt>> {
        let conn = self.read();
        let mut stmt = conn.prepare(
            "SELECT id, action, paths, detail, created, undone FROM receipts ORDER BY created DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Receipt {
                id: r.get(0)?,
                action: r.get(1)?,
                paths: r.get(2)?,
                detail: r.get(3)?,
                created: r.get(4)?,
                undone: r.get::<_, i64>(5)? != 0,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    // --------------------------------------------------------------- activity

    /// Record that the user opened a file. Files that are not indexed are
    /// ignored: without the guard the subselect yields NULL and SQLite silently
    /// turns it into a new rowid, building anonymous activity rows.
    pub fn record_open(&self, path: &Path) -> Result<()> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO activity(file_id, opens, last_open)
             SELECT (SELECT id FROM files WHERE path = ?1), 1, strftime('%s','now')
             WHERE EXISTS (SELECT 1 FROM files WHERE path = ?1)
             ON CONFLICT(file_id) DO UPDATE SET opens = opens + 1, last_open = strftime('%s','now')",
            params![path_str(path)],
        )?;
        Ok(())
    }

    pub fn clear_activity(&self) -> Result<usize> {
        Ok(self.lock().execute("DELETE FROM activity", [])?)
    }

    pub fn analyze(&self) -> Result<()> {
        self.lock().execute("ANALYZE", [])?;
        Ok(())
    }
}

// ------------------------------------------------------------------ internals

struct SortKey {
    col: &'static str,
    direction: Dir,
    /// Column read back from the row to build the next cursor.
    read: fn(&FileMeta) -> Value,
}

#[derive(Clone, Copy, PartialEq)]
enum Dir {
    Desc,
    Asc,
}

impl SortKey {
    fn of(sort: Sort) -> SortKey {
        match sort {
            Sort::Modified => {
                SortKey { col: "f.mtime", direction: Dir::Desc, read: |f| f.mtime.into() }
            }
            Sort::Added => {
                SortKey { col: "f.added_at", direction: Dir::Desc, read: |f| f.added_at.into() }
            }
            Sort::Size => SortKey { col: "f.size", direction: Dir::Desc, read: |f| f.size.into() },
            Sort::Name => SortKey {
                col: "f.name COLLATE NOCASE",
                direction: Dir::Asc,
                read: |f| f.name.clone().into(),
            },
            Sort::Kind => {
                SortKey { col: "f.kind", direction: Dir::Asc, read: |f| f.kind.as_str().into() }
            }
            Sort::Frecency => SortKey {
                col: "CAST(COALESCE(a.opens, 0) * 1000.0 / (1.0 + MAX(0, strftime('%s','now') - COALESCE(a.last_open, 0)) / 2592000.0) AS INTEGER)",
                direction: Dir::Desc,
                read: |f| f.frecency.into(),
            },
        }
    }

    fn key_of(&self, f: &FileMeta) -> Value {
        (self.read)(f)
    }
}

fn base_select(extra_join: &str) -> String {
    format!(
        "SELECT f.id, f.path, f.parent, f.name, f.ext, f.kind, f.size, f.mtime, f.added_at,
                f.is_screenshot, s.url, s.app, s.host, f.in_trash, f.trash_path,
                f.duration_ms, COALESCE(a.opens, 0),
                CAST(COALESCE(a.opens, 0) * 1000.0 / (1.0 + MAX(0, strftime('%s','now') - COALESCE(a.last_open, 0)) / 2592000.0) AS INTEGER) AS frecency
         FROM files f
         LEFT JOIN sources s ON s.id = f.source_id
         LEFT JOIN activity a ON a.file_id = f.id{extra_join}"
    )
}

fn map_meta(r: &Row) -> rusqlite::Result<FileMeta> {
    let kind: String = r.get(5)?;
    Ok(FileMeta {
        id: r.get(0)?,
        path: r.get(1)?,
        parent: r.get(2)?,
        name: r.get(3)?,
        ext: r.get(4)?,
        kind: FileKind::ALL
            .into_iter()
            .chain([FileKind::Other])
            .find(|k| k.as_str() == kind)
            .unwrap_or(FileKind::Other),
        size: r.get::<_, i64>(6)? as u64,
        mtime: r.get(7)?,
        added_at: r.get(8)?,
        is_screenshot: r.get::<_, i64>(9)? != 0,
        origin_url: r.get(10)?,
        origin_app: r.get(11)?,
        origin_host: r.get(12)?,
        in_trash: r.get::<_, i64>(13)? != 0,
        trash_path: r.get(14)?,
        duration_ms: r.get::<_, Option<i64>>(15)?.map(|v| v.max(0) as u64),
        opens: r.get::<_, i64>(16).unwrap_or(0) as u64,
        frecency: r.get::<_, i64>(17).unwrap_or(0),
    })
}

fn upsert_one(tx: &Transaction, f: &ScannedFile, generation: Option<i64>) -> Result<usize> {
    let path = path_str(&f.path);
    let name = f.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let parent = f.path.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = f.path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let changed = tx.execute(
        "INSERT INTO files(path, parent, name, ext, kind, size, mtime, added_at, inode, is_screenshot, scan_gen, duration_ms)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, COALESCE(NULLIF(?10, 0), ?7), ?8, ?9, ?11, ?12)
         ON CONFLICT(path) DO UPDATE SET
           parent = excluded.parent, name = excluded.name, ext = excluded.ext,
           kind = excluded.kind, size = excluded.size, mtime = excluded.mtime,
           inode = excluded.inode, is_screenshot = excluded.is_screenshot,
           scan_gen = MAX(files.scan_gen, excluded.scan_gen),
           duration_ms = COALESCE(excluded.duration_ms, files.duration_ms),
           in_trash = 0, trash_path = NULL
         WHERE excluded.parent <> files.parent OR excluded.name <> files.name
            OR excluded.ext <> files.ext OR excluded.kind <> files.kind
            OR excluded.size <> files.size OR excluded.mtime <> files.mtime
            OR excluded.is_screenshot <> files.is_screenshot
            OR files.in_trash <> 0
            OR files.scan_gen < excluded.scan_gen
            OR files.duration_ms IS NOT excluded.duration_ms
            OR excluded.inode IS NOT files.inode",
        params![
            path,
            parent,
            name,
            ext,
            f.kind.as_str(),
            f.size as i64,
            f.mtime,
            f.inode.map(|i| i as i64),
            f.is_screenshot as i64,
            f.created,
            generation.unwrap_or(0),
            f.duration_ms.map(|d| d as i64),
        ],
    )?;
    Ok(changed)
}

pub(crate) fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn fts_phrase(term: &str) -> String {
    let cleaned: String =
        term.chars().filter(|c| !matches!(c, '"' | '(' | ')' | '*' | ':')).collect();
    format!("\"{cleaned}\"")
}

fn encode_cursor(key: &Value, id: i64) -> Result<String> {
    Ok(serde_json::to_string(&[key.clone(), id.into()])?)
}

fn decode_cursor(cursor: &str) -> Result<(CursorKey, i64)> {
    let v: Vec<Value> = serde_json::from_str(cursor).map_err(|_| Error::InvalidCursor)?;
    if v.len() != 2 {
        return Err(Error::InvalidCursor);
    }
    let id = v[1].as_i64().ok_or(Error::InvalidCursor)?;
    let key = match &v[0] {
        Value::Number(n) => CursorKey::Int(n.as_i64().ok_or(Error::InvalidCursor)?),
        Value::String(s) => CursorKey::Str(s.clone()),
        _ => return Err(Error::InvalidCursor),
    };
    Ok((key, id))
}

#[derive(Clone)]
enum CursorKey {
    Int(i64),
    Str(String),
}

impl rusqlite::types::ToSql for CursorKey {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            CursorKey::Int(v) => v.to_sql(),
            CursorKey::Str(v) => v.to_sql(),
        }
    }
}

fn path_str(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

const SCHEMA_V1: &str = r#"
CREATE TABLE files (
  id INTEGER PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,
  parent TEXT NOT NULL,
  name TEXT NOT NULL,
  ext TEXT NOT NULL DEFAULT '',
  kind TEXT NOT NULL,
  size INTEGER NOT NULL,
  mtime INTEGER NOT NULL,
  added_at INTEGER NOT NULL,
  inode INTEGER,
  is_screenshot INTEGER NOT NULL DEFAULT 0,
  source_id INTEGER REFERENCES sources(id) ON DELETE SET NULL,
  in_trash INTEGER NOT NULL DEFAULT 0,
  trash_path TEXT
);
CREATE INDEX files_kind ON files(kind) WHERE in_trash = 0;
CREATE INDEX files_mtime ON files(in_trash, mtime DESC, id DESC);
CREATE INDEX files_size ON files(in_trash, size DESC, id DESC);
CREATE INDEX files_added ON files(in_trash, added_at DESC, id DESC);
CREATE INDEX files_parent ON files(parent);

CREATE TABLE sources (
  id INTEGER PRIMARY KEY,
  url TEXT NOT NULL UNIQUE,
  app TEXT,
  host TEXT
);

CREATE TABLE tags (
  file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  tag TEXT NOT NULL,
  PRIMARY KEY (file_id, tag)
);

CREATE TABLE saved_searches (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  query TEXT NOT NULL,
  created INTEGER NOT NULL
);

CREATE TABLE receipts (
  id INTEGER PRIMARY KEY,
  action TEXT NOT NULL,
  paths TEXT NOT NULL,
  detail TEXT,
  created INTEGER NOT NULL,
  undone INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE activity (
  file_id INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,
  opens INTEGER NOT NULL DEFAULT 0,
  last_open INTEGER
);

CREATE VIRTUAL TABLE files_fts USING fts5(name, content='files', content_rowid='id', tokenize='trigram');
CREATE TRIGGER files_fts_ai AFTER INSERT ON files BEGIN
  INSERT INTO files_fts(rowid, name) VALUES (new.id, new.name);
END;
CREATE TRIGGER files_fts_ad AFTER DELETE ON files BEGIN
  INSERT INTO files_fts(files_fts, rowid, name) VALUES ('delete', old.id, old.name);
END;
CREATE TRIGGER files_fts_au AFTER UPDATE OF name ON files BEGIN
  INSERT INTO files_fts(files_fts, rowid, name) VALUES ('delete', old.id, old.name);
  INSERT INTO files_fts(rowid, name) VALUES (new.id, new.name);
END;
"#;
