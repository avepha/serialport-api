use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{params, Connection, OptionalExtension};

use super::{
    validate_request, CreatePreset, Preset, PresetStore, PresetStoreError, PresetStoreResult,
};

#[derive(Clone)]
pub struct SqlitePresetStore {
    connection: Arc<Mutex<Connection>>,
}

impl SqlitePresetStore {
    pub fn open(path: impl AsRef<Path>) -> PresetStoreResult<Self> {
        let connection = Connection::open(path).map_err(sqlite_error)?;
        connection
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS presets (
                  id INTEGER PRIMARY KEY AUTOINCREMENT,
                  name TEXT NOT NULL,
                  payload_json TEXT NOT NULL,
                  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
                "#,
            )
            .map_err(sqlite_error)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    fn lock(&self) -> PresetStoreResult<std::sync::MutexGuard<'_, Connection>> {
        self.connection
            .lock()
            .map_err(|_| PresetStoreError::Storage("sqlite connection lock poisoned".to_string()))
    }
}

impl PresetStore for SqlitePresetStore {
    fn list(&self) -> PresetStoreResult<Vec<Preset>> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare("SELECT id, name, payload_json FROM presets ORDER BY id ASC")
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map([], row_to_preset)
            .map_err(sqlite_error)?;
        rows.map(|row| row.map_err(sqlite_error)).collect()
    }

    fn create(&self, request: CreatePreset) -> PresetStoreResult<Preset> {
        let (name, payload) = validate_request(request)?;
        let payload_json = serde_json::to_string(&payload).map_err(json_error)?;
        let connection = self.lock()?;
        connection
            .execute(
                "INSERT INTO presets (name, payload_json) VALUES (?1, ?2)",
                params![name, payload_json],
            )
            .map_err(sqlite_error)?;
        let id = connection.last_insert_rowid();
        Ok(Preset { id, name, payload })
    }

    fn get(&self, id: i64) -> PresetStoreResult<Preset> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT id, name, payload_json FROM presets WHERE id = ?1",
                params![id],
                row_to_preset,
            )
            .optional()
            .map_err(sqlite_error)?
            .ok_or(PresetStoreError::NotFound(id))
    }

    fn update(&self, id: i64, request: CreatePreset) -> PresetStoreResult<Preset> {
        let (name, payload) = validate_request(request)?;
        let payload_json = serde_json::to_string(&payload).map_err(json_error)?;
        let connection = self.lock()?;
        let changed = connection
            .execute(
                "UPDATE presets SET name = ?1, payload_json = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
                params![name, payload_json, id],
            )
            .map_err(sqlite_error)?;
        if changed == 0 {
            return Err(PresetStoreError::NotFound(id));
        }
        Ok(Preset { id, name, payload })
    }

    fn delete(&self, id: i64) -> PresetStoreResult<i64> {
        let connection = self.lock()?;
        let changed = connection
            .execute("DELETE FROM presets WHERE id = ?1", params![id])
            .map_err(sqlite_error)?;
        if changed == 0 {
            Err(PresetStoreError::NotFound(id))
        } else {
            Ok(id)
        }
    }
}

fn row_to_preset(row: &rusqlite::Row<'_>) -> rusqlite::Result<Preset> {
    let id = row.get(0)?;
    let name = row.get(1)?;
    let payload_json: String = row.get(2)?;
    let payload = serde_json::from_str(&payload_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(Preset { id, name, payload })
}

fn sqlite_error(error: rusqlite::Error) -> PresetStoreError {
    PresetStoreError::Storage(error.to_string())
}

fn json_error(error: serde_json::Error) -> PresetStoreError {
    PresetStoreError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{CreatePreset, PresetStore, PresetStoreError};
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn sqlite_store_persists_create_update_delete_across_reopen() {
        let path = unique_db_path("sqlite_store_persists_create_update_delete_across_reopen");
        let _ = std::fs::remove_file(&path);

        let store = SqlitePresetStore::open(&path).unwrap();
        let created = store
            .create(CreatePreset {
                name: "Persistent ping".to_string(),
                payload: json!({"method":"query","topic":"ping","data":{}}),
            })
            .unwrap();
        assert_eq!(created.id, 1);
        drop(store);

        let store = SqlitePresetStore::open(&path).unwrap();
        assert_eq!(store.list().unwrap(), vec![created.clone()]);
        let updated = store
            .update(
                1,
                CreatePreset {
                    name: "Persistent pong".to_string(),
                    payload: json!({"method":"query","topic":"pong","data":{}}),
                },
            )
            .unwrap();
        assert_eq!(updated.name, "Persistent pong");
        drop(store);

        let store = SqlitePresetStore::open(&path).unwrap();
        assert_eq!(store.get(1).unwrap(), updated);
        assert_eq!(store.delete(1).unwrap(), 1);
        drop(store);

        let store = SqlitePresetStore::open(&path).unwrap();
        assert!(matches!(store.get(1), Err(PresetStoreError::NotFound(1))));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn sqlite_store_enforces_validation() {
        let path = unique_db_path("sqlite_store_enforces_validation");
        let _ = std::fs::remove_file(&path);
        let store = SqlitePresetStore::open(&path).unwrap();

        assert!(matches!(
            store.create(CreatePreset {
                name: "".to_string(),
                payload: json!({}),
            }),
            Err(PresetStoreError::InvalidName)
        ));
        assert!(matches!(
            store.create(CreatePreset {
                name: "Bad payload".to_string(),
                payload: json!("not object"),
            }),
            Err(PresetStoreError::InvalidPayload)
        ));
        let _ = std::fs::remove_file(&path);
    }

    fn unique_db_path(test_name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "serialport-api-phase13-{test_name}-{}.db",
            std::process::id()
        ))
    }
}
