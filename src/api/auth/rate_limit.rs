use crate::api::AppState;
use sha2::{Digest, Sha256};
use warp::{reply::Response, Reply};

use super::auth_error;

fn login_attempt_key(email: &str) -> String {
    let normalized = email.trim().to_lowercase();
    let digest = Sha256::digest(normalized.as_bytes());
    format!("auth:login-attempts:{digest:x}")
}

pub(super) async fn login_is_rate_limited(email: &str, state: &AppState) -> bool {
    let key = login_attempt_key(email);
    let mut connection = state.redis.clone();
    match redis::cmd("GET")
        .arg(key)
        .query_async::<Option<u32>>(&mut connection)
        .await
    {
        Ok(Some(attempts)) => attempts >= state.config.security.max_login_attempts.max(1),
        Ok(None) => false,
        Err(error) => {
            tracing::error!(%error, "Impossible de lire le compteur anti-bruteforce Redis");
            false
        }
    }
}

async fn record_failed_login(email: &str, state: &AppState) -> u32 {
    let key = login_attempt_key(email);
    let lockout_seconds = state
        .config
        .security
        .lockout_duration_minutes
        .max(1)
        .saturating_mul(60)
        .min(i64::MAX as u64) as i64;
    let mut connection = state.redis.clone();
    let result = redis::pipe()
        .atomic()
        .cmd("INCR")
        .arg(&key)
        .cmd("EXPIRE")
        .arg(&key)
        .arg(lockout_seconds)
        .query_async::<(u32, bool)>(&mut connection)
        .await;

    match result {
        Ok((attempts, _)) => attempts,
        Err(error) => {
            tracing::error!(%error, "Impossible de mettre à jour le compteur anti-bruteforce Redis");
            0
        }
    }
}

pub(super) async fn clear_login_failures(email: &str, state: &AppState) {
    let key = login_attempt_key(email);
    let mut connection = state.redis.clone();
    if let Err(error) = redis::cmd("DEL")
        .arg(key)
        .query_async::<u32>(&mut connection)
        .await
    {
        tracing::error!(%error, "Impossible de réinitialiser le compteur anti-bruteforce Redis");
    }
}

pub(super) async fn login_failure_response(email: &str, state: &AppState) -> Response {
    let attempts = record_failed_login(email, state).await;
    if attempts >= state.config.security.max_login_attempts.max(1) {
        rate_limit_error(state)
    } else {
        auth_error(
            "Identifiants invalides",
            warp::http::StatusCode::UNAUTHORIZED,
        )
        .into_response()
    }
}

pub(super) fn rate_limit_error(state: &AppState) -> Response {
    let retry_after_seconds = state
        .config
        .security
        .lockout_duration_minutes
        .max(1)
        .saturating_mul(60);
    warp::reply::with_header(
        warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Trop de tentatives de connexion",
                "retry_after_seconds": retry_after_seconds
            })),
            warp::http::StatusCode::TOO_MANY_REQUESTS,
        ),
        warp::http::header::RETRY_AFTER,
        retry_after_seconds,
    )
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::login_attempt_key;

    #[test]
    fn login_attempt_keys_are_normalized_and_do_not_expose_email_addresses() {
        let lower = login_attempt_key("admin@example.test");
        let mixed = login_attempt_key("  Admin@Example.Test ");
        assert_eq!(lower, mixed);
        assert!(!lower.contains("admin@example.test"));
    }
}
