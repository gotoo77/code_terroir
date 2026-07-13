use sqlx::PgPool;
use std::time::Duration;
use tracing::info;

#[derive(Debug, Clone)]
pub struct DatabasePool {
    pub pool: PgPool,
}

impl DatabasePool {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        info!("🔗 Connecting to database...");

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(20)
            .min_connections(2)
            .acquire_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(3600))
            .connect(database_url)
            .await?;

        // Test de connexion
        sqlx::query("SELECT 1").execute(&pool).await?;

        info!("✅ Database connection established");

        Ok(Self { pool })
    }

    #[allow(dead_code)]
    /// Execute a health check query
    pub async fn health_check(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}
