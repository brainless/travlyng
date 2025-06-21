use rusqlite::{Connection, Result};
use std::fs;
use std::sync::Mutex;

// Database initialization (moved Data struct here for simplicity)
pub struct AppState {
    pub db: Mutex<Connection>,
}

pub fn init_db() -> Result<Connection> {
    let conn = Connection::open("travel_planner.db")?;
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let schema_path = std::path::Path::new(manifest_dir).join("schema.sql");
    let schema = fs::read_to_string(schema_path)
        .expect("Should have been able to read the file");
    conn.execute_batch(&schema)?;
    println!("Database initialized successfully.");
    Ok(conn)
}
