pub mod sqlite;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub id: i64,
    pub name: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CreatePreset {
    pub name: String,
    pub payload: Value,
}

#[derive(Debug, Error)]
pub enum PresetStoreError {
    #[error("preset name must not be empty")]
    InvalidName,
    #[error("preset payload must be a JSON object")]
    InvalidPayload,
    #[error("preset not found: {0}")]
    NotFound(i64),
    #[error("preset storage error: {0}")]
    Storage(String),
}

pub type PresetStoreResult<T> = std::result::Result<T, PresetStoreError>;

pub trait PresetStore: Send + Sync + 'static {
    fn list(&self) -> PresetStoreResult<Vec<Preset>>;
    fn create(&self, request: CreatePreset) -> PresetStoreResult<Preset>;
    fn get(&self, id: i64) -> PresetStoreResult<Preset>;
    fn update(&self, id: i64, request: CreatePreset) -> PresetStoreResult<Preset>;
    fn delete(&self, id: i64) -> PresetStoreResult<i64>;
}

#[derive(Clone, Default)]
pub struct InMemoryPresetStore {
    state: Arc<Mutex<InMemoryPresetState>>,
}

#[derive(Default)]
struct InMemoryPresetState {
    next_id: i64,
    presets: BTreeMap<i64, Preset>,
}

impl PresetStore for InMemoryPresetStore {
    fn list(&self) -> PresetStoreResult<Vec<Preset>> {
        let state = self
            .state
            .lock()
            .map_err(|_| PresetStoreError::Storage("preset store lock poisoned".to_string()))?;
        Ok(state.presets.values().cloned().collect())
    }

    fn create(&self, request: CreatePreset) -> PresetStoreResult<Preset> {
        let (name, payload) = validate_request(request)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| PresetStoreError::Storage("preset store lock poisoned".to_string()))?;
        let id = state.next_id + 1;
        state.next_id = id;
        let preset = Preset { id, name, payload };
        state.presets.insert(id, preset.clone());
        Ok(preset)
    }

    fn get(&self, id: i64) -> PresetStoreResult<Preset> {
        let state = self
            .state
            .lock()
            .map_err(|_| PresetStoreError::Storage("preset store lock poisoned".to_string()))?;
        state
            .presets
            .get(&id)
            .cloned()
            .ok_or(PresetStoreError::NotFound(id))
    }

    fn update(&self, id: i64, request: CreatePreset) -> PresetStoreResult<Preset> {
        let (name, payload) = validate_request(request)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| PresetStoreError::Storage("preset store lock poisoned".to_string()))?;
        let preset = state
            .presets
            .get_mut(&id)
            .ok_or(PresetStoreError::NotFound(id))?;
        preset.name = name;
        preset.payload = payload;
        Ok(preset.clone())
    }

    fn delete(&self, id: i64) -> PresetStoreResult<i64> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| PresetStoreError::Storage("preset store lock poisoned".to_string()))?;
        state
            .presets
            .remove(&id)
            .map(|_| id)
            .ok_or(PresetStoreError::NotFound(id))
    }
}

pub fn validate_request(request: CreatePreset) -> PresetStoreResult<(String, Value)> {
    let name = request.name.trim().to_string();
    if name.is_empty() {
        return Err(PresetStoreError::InvalidName);
    }
    if !request.payload.is_object() {
        return Err(PresetStoreError::InvalidPayload);
    }
    Ok((name, request.payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    #[test]
    fn in_memory_store_creates_lists_validates_and_reports_missing() {
        let store = InMemoryPresetStore::default();

        let preset = store
            .create(CreatePreset {
                name: "Read IMU".to_string(),
                payload: json!({"method":"query","topic":"imu.read","data":{}}),
            })
            .unwrap();
        assert_eq!(preset.id, 1);
        assert_eq!(store.list().unwrap(), vec![preset.clone()]);
        assert_eq!(store.get(1).unwrap(), preset);

        let updated = store
            .update(
                1,
                CreatePreset {
                    name: "Read temperature".to_string(),
                    payload: json!({"method":"query","topic":"temperature.read","data":{}}),
                },
            )
            .unwrap();
        assert_eq!(updated.name, "Read temperature");
        assert_eq!(store.delete(1).unwrap(), 1);

        assert!(matches!(
            store.create(CreatePreset {
                name: "   ".to_string(),
                payload: json!({}),
            }),
            Err(PresetStoreError::InvalidName)
        ));
        assert!(matches!(
            store.create(CreatePreset {
                name: "Bad".to_string(),
                payload: json!([]),
            }),
            Err(PresetStoreError::InvalidPayload)
        ));
        assert!(matches!(store.get(99), Err(PresetStoreError::NotFound(99))));
        assert!(matches!(
            store.update(
                99,
                CreatePreset {
                    name: "Missing".to_string(),
                    payload: json!({}),
                },
            ),
            Err(PresetStoreError::NotFound(99))
        ));
        assert!(matches!(
            store.delete(99),
            Err(PresetStoreError::NotFound(99))
        ));
    }
}
