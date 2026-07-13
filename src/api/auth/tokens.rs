use crate::api::AppState;
use crate::models::user::{LoginResponse, User, UserProfile};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use super::{auth_error, TokenClaims, TokenIssueError};

pub(super) async fn issue_login_response(
    user: &User,
    state: &AppState,
) -> Result<LoginResponse, TokenIssueError> {
    let session_id = Uuid::new_v4();
    let response = build_login_response(user, state, session_id)?;

    sqlx::query(
        r#"
        INSERT INTO auth_sessions (id, family_id, user_id, expires_at)
        VALUES ($1, $1, $2, $3)
        "#,
    )
    .bind(session_id)
    .bind(user.id)
    .bind(refresh_expiry(state))
    .execute(&state.db.pool)
    .await?;

    Ok(response)
}

pub(super) fn build_login_response(
    user: &User,
    state: &AppState,
    session_id: Uuid,
) -> Result<LoginResponse, jsonwebtoken::errors::Error> {
    let access_token = encode_token(
        user,
        "access",
        state.config.jwt_expiration_hours,
        &state.config.jwt_secret,
        session_id,
    )?;
    let refresh_token = encode_token(
        user,
        "refresh",
        refresh_validity_hours(state),
        &state.config.jwt_secret,
        session_id,
    )?;

    Ok(LoginResponse {
        access_token,
        refresh_token,
        expires_in: state
            .config
            .jwt_expiration_hours
            .saturating_mul(3600)
            .min(i64::MAX as u64) as i64,
        user: UserProfile {
            id: user.id,
            email: user.email.clone(),
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            role: user.role.clone(),
            producer_id: user.producer_id,
        },
    })
}

fn encode_token(
    user: &User,
    token_type: &str,
    validity_hours: u64,
    jwt_secret: &str,
    session_id: Uuid,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let claims = TokenClaims {
        sub: user.id.to_string(),
        jti: session_id.to_string(),
        producer_id: user.producer_id.to_string(),
        role: user.role.clone(),
        token_type: token_type.to_string(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::hours(validity_hours.min(i64::MAX as u64) as i64)).timestamp()
            as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
}

pub(super) fn refresh_expiry(state: &AppState) -> DateTime<Utc> {
    Utc::now() + Duration::hours(refresh_validity_hours(state) as i64)
}

fn refresh_validity_hours(state: &AppState) -> u64 {
    state
        .config
        .jwt_expiration_hours
        .saturating_mul(24)
        .min(i64::MAX as u64)
}

pub(super) async fn revoke_session_family(
    family_id: Uuid,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE auth_sessions SET revoked_at = COALESCE(revoked_at, NOW()) WHERE family_id = $1",
    )
    .bind(family_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

pub(super) fn decode_token(
    token: &str,
    expected_type: &str,
    state: &AppState,
) -> Result<TokenClaims, warp::reply::WithStatus<warp::reply::Json>> {
    decode_claims(token, expected_type, &state.config.jwt_secret)
        .map_err(|_| auth_error("Token invalide", warp::http::StatusCode::UNAUTHORIZED))
}

pub(super) fn decode_claims(
    token: &str,
    expected_type: &str,
    jwt_secret: &str,
) -> Result<TokenClaims, jsonwebtoken::errors::Error> {
    let decoded = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )?;

    if decoded.claims.token_type != expected_type {
        return Err(jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken,
        ));
    }

    Ok(decoded.claims)
}
