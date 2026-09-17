use crate::model::{Settings, DEFAULT_CLIENT_ID};
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

pub struct Store(Connection);
impl Store {
    pub fn open(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
        let conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(3))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS preferences (id INTEGER PRIMARY KEY CHECK(id=1), json TEXT NOT NULL);")?;
        Ok(Self(conn))
    }
    pub fn load(&self) -> Result<Settings, Box<dyn std::error::Error>> {
        let json: Option<String> = self.0.query_row("SELECT json FROM preferences WHERE id=1", [], |row| row.get(0)).optional()?;
        let mut value: Settings = json.map(|s| serde_json::from_str(&s)).transpose()?.unwrap_or_default();
        // Upgrade earlier POC preferences without overwriting a custom application ID.
        // The effective default is persisted on the next successful settings save.
        if value.client_id.trim().is_empty() {
            value.client_id = DEFAULT_CLIENT_ID.to_owned();
        }
        value.validate().map_err(std::io::Error::other)?;
        Ok(value)
    }
    pub fn save(&mut self, settings: &Settings) -> Result<(), String> {
        settings.validate()?;
        let json = serde_json::to_string(settings).map_err(|e| e.to_string())?;
        let tx = self.0.transaction().map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO preferences(id,json) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET json=excluded.json", [&json]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn memory_store(json: Option<&str>) -> Store {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE preferences (id INTEGER PRIMARY KEY CHECK(id=1), json TEXT NOT NULL);",
        ).unwrap();
        if let Some(json) = json {
            conn.execute("INSERT INTO preferences(id,json) VALUES(1,?1)", [json]).unwrap();
        }
        Store(conn)
    }

    #[test]
    fn fresh_install_uses_registered_client_id() {
        let settings = memory_store(None).load().unwrap();
        assert_eq!(settings.client_id, DEFAULT_CLIENT_ID);
        assert_eq!(settings.limit, 3);
        assert!(settings.demo);
    }

    #[test]
    fn missing_client_id_uses_registered_default() {
        let settings = memory_store(Some(r#"{"schema":1,"volume":40}"#)).load().unwrap();
        assert_eq!(settings.client_id, DEFAULT_CLIENT_ID);
        assert_eq!(settings.volume, 40);
    }

    #[test]
    fn empty_legacy_client_id_is_upgraded_without_resetting_preferences() {
        let settings = memory_store(Some(
            r#"{"client_id":"","limit":2,"volume":40,"muted":true,"demo":false,"favorites":[{"login":"modpackdad","enabled":true}]}"#,
        )).load().unwrap();
        assert_eq!(settings.client_id, DEFAULT_CLIENT_ID);
        assert_eq!(settings.limit, 2);
        assert_eq!(settings.volume, 40);
        assert!(settings.muted);
        assert!(!settings.demo);
        assert_eq!(settings.favorites.len(), 1);
        assert_eq!(settings.favorites[0].login, "modpackdad");
    }

    #[test]
    fn whitespace_legacy_client_id_is_upgraded() {
        let settings = memory_store(Some(r#"{"client_id":"   "}"#)).load().unwrap();
        assert_eq!(settings.client_id, DEFAULT_CLIENT_ID);
    }

    #[test]
    fn custom_client_id_is_preserved() {
        let settings = memory_store(Some(
            r#"{"client_id":"custompublicclientid123"}"#,
        )).load().unwrap();
        assert_eq!(settings.client_id, "custompublicclientid123");
    }

    #[test]
    fn upgraded_client_id_is_persisted_on_save() {
        let mut store = memory_store(Some(r#"{"client_id":""}"#));
        let settings = store.load().unwrap();
        store.save(&settings).unwrap();
        let json: String = store.0.query_row(
            "SELECT json FROM preferences WHERE id=1", [], |row| row.get(0),
        ).unwrap();
        let persisted: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(persisted.client_id, DEFAULT_CLIENT_ID);
        assert_eq!(store.load().unwrap().client_id, DEFAULT_CLIENT_ID);
    }
}
