pub mod models;

use std::str::FromStr;
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;

/// Initialize a SQLite connection pool.
///
/// WAL mode plus a busy timeout is required because the proxy writes a request log and
/// updates quota counters on every call; without them concurrent requests hit
/// "database is locked" errors.
pub async fn init_pool(path: &str, pool_size: u32) -> SqlitePool {
    // Ensure the database file exists (SQLx SQLite doesn't auto-create by default)
    if !std::path::Path::new(path).exists() {
        if let Some(parent) = std::path::Path::new(path).parent()
            && !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .unwrap_or_else(|e| panic!("Failed to create database directory: {e}"));
            }
        std::fs::File::create(path)
            .unwrap_or_else(|e| panic!("Failed to create database file: {e}"));
    }

    let options = SqliteConnectOptions::from_str(&format!("sqlite://{path}"))
        .unwrap_or_else(|e| panic!("Invalid database path '{path}': {e}"))
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(30))
        .foreign_keys(true)
        .create_if_missing(true);

    SqlitePoolOptions::new()
        .max_connections(pool_size.max(1))
        .acquire_timeout(Duration::from_secs(30))
        .connect_with(options)
        .await
        .expect("Failed to connect to database")
}

/// Run all pending SQLx migrations.
pub async fn run_migrations(pool: &SqlitePool) {
    sqlx::migrate!("src/db/migrations")
        .run(pool)
        .await
        .expect("Failed to run database migrations");
}

/// Ensure the admin user exists; create one if not present.
pub async fn ensure_admin(pool: &SqlitePool, username: &str, password: &str) {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM users WHERE username = ? AND role = 'admin')",
    )
    .bind(username)
    .fetch_one(pool)
    .await
    .expect("Failed to check admin existence");

    if !exists {
        let password_hash =
            bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash admin password");
        let api_key = format!("sk-{}", uuid::Uuid::new_v4().simple());

        let mut tx = pool.begin().await.expect("Failed to start admin tx");

        let user_id: i64 = sqlx::query_scalar(
            "INSERT INTO users (username, password_hash, role, api_key, status) VALUES (?, ?, 'admin', ?, 'active') RETURNING id",
        )
        .bind(username)
        .bind(&password_hash)
        .bind(&api_key)
        .fetch_one(&mut *tx)
        .await
        .expect("Failed to create admin user");

        // Also create an initial user_key so the admin can see it in the Key Management page
        sqlx::query(
            "INSERT INTO user_keys (user_id, key_value, name, status) VALUES (?, ?, 'Default Key', 'active')",
        )
        .bind(user_id)
        .bind(&api_key)
        .execute(&mut *tx)
        .await
        .expect("Failed to create initial admin key");

        tx.commit().await.expect("Failed to commit admin creation");

        tracing::info!(username, "Initial admin user created");
    } else {
        tracing::info!(username, "Admin user already exists");
    }
}
