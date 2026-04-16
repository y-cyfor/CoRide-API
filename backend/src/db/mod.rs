pub mod models;

use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

/// Initialize a SQLite connection pool.
pub async fn init_pool(path: &str, pool_size: u32) -> SqlitePool {
    // Ensure the database file exists (SQLx SQLite doesn't auto-create by default)
    if !std::path::Path::new(path).exists() {
        // Create parent directories if needed
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("Failed to create database directory: {e}"));
        }
        // Create empty database file
        std::fs::File::create(path)
            .unwrap_or_else(|e| panic!("Failed to create database file: {e}"));
    }

    SqlitePoolOptions::new()
        .max_connections(pool_size)
        .connect(path)
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

        sqlx::query(
            "INSERT INTO users (username, password_hash, role, api_key, status) VALUES (?, ?, 'admin', ?, 'active')",
        )
        .bind(username)
        .bind(&password_hash)
        .bind(&api_key)
        .execute(pool)
        .await
        .expect("Failed to create admin user");

        tracing::info!(username, "Initial admin user created");
    } else {
        tracing::info!(username, "Admin user already exists");
    }
}
