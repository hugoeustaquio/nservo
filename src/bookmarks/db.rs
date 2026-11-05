use std::path::Path;
use rusqlite::{params, Connection};
use crate::bookmarks::entry::BookmarkEntry;
use crate::error::Result;

pub struct BookmarksDatabase {
    conn: Connection,
}

impl BookmarksDatabase {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS bookmarks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                tags TEXT NOT NULL DEFAULT '',
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_bookmarks_url ON bookmarks(url);",
            [],
        )?;
        Ok(())
    }

    pub fn add(&self, url: &str, title: &str, tags: &[String]) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        let tags_str = tags.join(",");
        self.conn.execute(
            "INSERT INTO bookmarks (url, title, tags, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(url) DO UPDATE SET
                title = ?2,
                tags = ?3",
            params![url, title, tags_str, now],
        )?;
        Ok(())
    }

    pub fn remove(&self, url: &str) -> Result<bool> {
        let count = self.conn.execute("DELETE FROM bookmarks WHERE url = ?1", params![url])?;
        Ok(count > 0)
    }

    pub fn search(&self, query: &str) -> Result<Vec<BookmarkEntry>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, tags, created_at
             FROM bookmarks
             WHERE url LIKE ?1 OR title LIKE ?1 OR tags LIKE ?1
             ORDER BY title ASC",
        )?;

        let rows = stmt.query_map(params![pattern], |row| {
            let tags_str: String = row.get(3)?;
            let tags = tags_str.split(',').filter(|s| !s.is_empty()).map(String::from).collect();
            Ok(BookmarkEntry::new(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                tags,
                row.get(4)?,
            ))
        })?;

        let mut results = Vec::new();
        for item in rows {
            results.push(item?);
        }
        Ok(results)
    }

    pub fn list_all(&self) -> Result<Vec<BookmarkEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, title, tags, created_at
             FROM bookmarks
             ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            let tags_str: String = row.get(3)?;
            let tags = tags_str.split(',').filter(|s| !s.is_empty()).map(String::from).collect();
            Ok(BookmarkEntry::new(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                tags,
                row.get(4)?,
            ))
        })?;

        let mut results = Vec::new();
        for item in rows {
            results.push(item?);
        }
        Ok(results)
    }

    pub fn clear(&self) -> Result<()> {
        self.conn.execute("DELETE FROM bookmarks", [])?;
        Ok(())
    }
}
