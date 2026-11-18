//! Script de validação do bootstrap desktop
//! Executado via `cargo test --no-run` ou separado

use tina::TinaApp;

#[tauri::command]
pub fn validate_bootstrap(app: tauri::State<'_, TinaApp>) -> Result<serde_json::Value, String> {
    use serde_json::json;
    
    let pool = app.database().pool();
    
    // Test 1: Database connection
    let conn_test = sqlx::query("SELECT 1 as test")
        .fetch_one::<serde_json::Value>(pool)
        .await
        .map(|r| r["test"] == 1)
        .map_err(|e| e.to_string())?;
    
    // Test 2: Migrations applied
    let migrations_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    
    // Test 3: WAL mode
    let journal_mode = sqlx::query_scalar::<_, String>(
        "PRAGMA journal_mode"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    
    // Test 4: Foreign keys
    let fk_enabled = sqlx::query_scalar::<_, i64>(
        "PRAGMA foreign_keys"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    
    // Test 5: Foreign key integrity
    let fk_check = sqlx::query("PRAGMA foreign_key_check")
        .fetch_all::<serde_json::Value>(pool)
        .await
        .map(|rows| rows.is_empty())
        .map_err(|e| e.to_string())?;
    
    Ok(json!({
        "database_connected": conn_test,
        "tables_created": migrations_count,
        "wal_mode": journal_mode.to_uppercase() == "WAL",
        "foreign_keys_enabled": fk_enabled == 1,
        "foreign_keys_integrity": fk_check,
        "data_dir": app.config().data_dir.to_string_lossy(),
        "database_path": app.config().database_path.to_string_lossy()
    }))
}