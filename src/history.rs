use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

static DB: Mutex<Option<Connection>> = Mutex::new(None);

fn db_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("muro")
        .join("history.db")
}

pub fn init() -> Result<()> {
    let conn = Connection::open(db_path())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT DEFAULT (datetime('now', 'localtime')),
            whisper_text TEXT NOT NULL,
            final_text TEXT,
            command TEXT NOT NULL
        );"
    )?;
    *DB.lock().unwrap() = Some(conn);
    Ok(())
}

pub fn save(whisper_text: &str, final_text: &str, command: &str) {
    if let Some(conn) = DB.lock().unwrap().as_ref() {
        let _ = conn.execute(
            "INSERT INTO history (whisper_text, final_text, command) VALUES (?1, ?2, ?3)",
            rusqlite::params![whisper_text, final_text, command],
        );
    }
}
