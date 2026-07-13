use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub server_port: u16,
    pub jwt_secret: String,
    pub jwt_expiration_hours: u64,
    pub bootstrap_token: Option<String>,
    pub cors_allowed_origins: Vec<String>,
    pub upload_dir: String,
    pub export_dir: String,
    pub base_url: String,
    pub qr_base_url: String, // URL de base pour les QR codes
    pub public_port: u16,    // Port pour les endpoints publics
    pub smtp: SmtpConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub password_min_length: usize,
    pub session_timeout_minutes: u64,
    pub max_login_attempts: u32,
    pub lockout_duration_minutes: u64,
    pub max_request_body_bytes: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            password_min_length: 8,
            session_timeout_minutes: 480, // 8 heures
            max_login_attempts: 5,
            lockout_duration_minutes: 30,
            max_request_body_bytes: 1_048_576,
        }
    }
}

impl SecurityConfig {
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let defaults = Self::default();
        Ok(Self {
            password_min_length: env::var("PASSWORD_MIN_LENGTH")
                .unwrap_or_else(|_| defaults.password_min_length.to_string())
                .parse()?,
            session_timeout_minutes: env::var("SESSION_TIMEOUT_MINUTES")
                .unwrap_or_else(|_| defaults.session_timeout_minutes.to_string())
                .parse()?,
            max_login_attempts: env::var("MAX_LOGIN_ATTEMPTS")
                .unwrap_or_else(|_| defaults.max_login_attempts.to_string())
                .parse()?,
            lockout_duration_minutes: env::var("LOCKOUT_DURATION_MINUTES")
                .unwrap_or_else(|_| defaults.lockout_duration_minutes.to_string())
                .parse()?,
            max_request_body_bytes: env::var("MAX_REQUEST_BODY_BYTES")
                .unwrap_or_else(|_| defaults.max_request_body_bytes.to_string())
                .parse()?,
        })
    }
}

impl AppConfig {
    pub async fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // Charger les variables d'environnement
        dotenv::dotenv().ok();

        let config = Self {
            database_url: env::var("DATABASE_URL")?,

            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),

            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3030".to_string())
                .parse()?,

            jwt_secret: env::var("JWT_SECRET")?,

            jwt_expiration_hours: env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()?,

            bootstrap_token: env::var("BOOTSTRAP_TOKEN")
                .ok()
                .filter(|value| !value.trim().is_empty()),

            cors_allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:8081".to_string())
                .split(',')
                .map(str::trim)
                .filter(|origin| !origin.is_empty())
                .map(str::to_owned)
                .collect(),

            upload_dir: env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string()),

            export_dir: env::var("EXPORT_DIR").unwrap_or_else(|_| "./exports".to_string()),

            base_url: env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3030".to_string()),

            qr_base_url: env::var("QR_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3030".to_string()),

            public_port: env::var("PUBLIC_PORT")
                .unwrap_or_else(|_| "3030".to_string())
                .parse()?,

            smtp: SmtpConfig {
                host: env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string()),
                port: env::var("SMTP_PORT")
                    .unwrap_or_else(|_| "587".to_string())
                    .parse()?,
                username: env::var("SMTP_USERNAME").unwrap_or_default(),
                password: env::var("SMTP_PASSWORD").unwrap_or_default(),
                from_email: env::var("SMTP_FROM_EMAIL")
                    .unwrap_or_else(|_| "noreply@code-terroir.com".to_string()),
                from_name: env::var("SMTP_FROM_NAME")
                    .unwrap_or_else(|_| "Code Terroir".to_string()),
            },

            security: SecurityConfig::load()?,
        };

        // Créer les répertoires s'ils n'existent pas
        tokio::fs::create_dir_all(&config.upload_dir).await?;
        tokio::fs::create_dir_all(&config.export_dir).await?;

        Ok(config)
    }
}
