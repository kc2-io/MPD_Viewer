use crate::model::Settings;
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
        let value: Settings = json.map(|s| serde_json::from_str(&s)).transpose()?.unwrap_or_default();
        // Legacy client_id is ignored by Settings; all user preferences remain intact.
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
    #[test]
    fn legacy_client_ids_are_discarded_without_resetting_preferences() {
        for id in [serde_json::Value::Null, serde_json::json!(""), serde_json::json!("custompublicclientid123")] {
            let mut store = Store::open(Path::new(":memory:")).unwrap();
            let json = serde_json::json!({"client_id":id,"schema":1,"favorites":[{"login":"modpackdad","enabled":false}],
                "limit":2,"volume":40,"muted":true,"demo":false}).to_string();
            store.0.execute("INSERT INTO preferences(id,json) VALUES(1,?1)", [&json]).unwrap();
            let settings=store.load().unwrap();
            assert_eq!(settings.limit,2); assert_eq!(settings.volume,40);
            assert!(settings.muted); assert!(!settings.demo);
            assert_eq!(settings.favorites[0].login,"modpackdad"); assert!(!settings.favorites[0].enabled);
            store.save(&settings).unwrap();
            let saved:String=store.0.query_row("SELECT json FROM preferences WHERE id=1",[],|r|r.get(0)).unwrap();
            assert!(!saved.contains("client_id"));
            assert_eq!(store.load().unwrap().volume,40);
        }
    }
    #[test]
    fn fresh_install_retains_existing_defaults() {
        let settings=Store::open(Path::new(":memory:")).unwrap().load().unwrap();
        assert_eq!(settings.limit,3); assert!(settings.demo); assert_eq!(settings.volume,25);
    }
}
