use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_ENTRIES: u32 = 50;

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
        // Garder seulement les 50 dernières entrées
        let _ = conn.execute(
            "DELETE FROM history WHERE id NOT IN (SELECT id FROM history ORDER BY id DESC LIMIT ?1)",
            rusqlite::params![MAX_ENTRIES],
        );
    }
}

/// Retourne les dernières entrées formatées pour lecture
pub fn recent(count: usize) -> String {
    let guard = DB.lock().unwrap();
    let Some(conn) = guard.as_ref() else {
        return "Historique indisponible.".to_string();
    };

    let mut stmt = match conn.prepare(
        "SELECT timestamp, final_text, command FROM history ORDER BY id DESC LIMIT ?1"
    ) {
        Ok(s) => s,
        Err(_) => return "Erreur lecture historique.".to_string(),
    };

    let rows = stmt.query_map(rusqlite::params![count as u32], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    });

    let mut lines = Vec::new();
    if let Ok(rows) = rows {
        for row in rows.flatten() {
            let (ts, text, cmd) = row;
            if !text.is_empty() {
                lines.push(format!("[{}] ({}) {}", ts, cmd, text));
            }
        }
    }

    if lines.is_empty() {
        "Aucun historique.".to_string()
    } else {
        lines.join("\n")
    }
}
