use crate::api::AppState;
use crate::models::user::{LoginRequest, LoginResponse, User, UserProfile};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::convert::Infallible;
use uuid::Uuid;
use warp::{Filter, Rejection, Reply};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisterRequest {
    producer_id: Option<Uuid>,
    producer: Option<BootstrapProducerRequest>,
    email: String,
    password: String,
    first_name: String,
    last_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BootstrapProducerRequest {
    raison_sociale: String,
    adresse: String,
    code_postal: String,
    ville: String,
    pays: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TokenClaims {
    sub: String,
    jti: String,
    producer_id: String,
    role: String,
    token_type: String,
    exp: usize,
    iat: usize,
}

#[derive(Debug, FromRow)]
struct AuthSession {
    user_id: Uuid,
    family_id: Uuid,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, thiserror::Error)]
enum TokenIssueError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

#[derive(Debug)]
pub(crate) struct AuthenticationRequired;

impl warp::reject::Reject for AuthenticationRequired {}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub producer_id: Uuid,
    pub role: crate::models::user::UserRole,
}

pub fn routes(state: AppState) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let api_prefix = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("auth"));

    let login = api_prefix
        .and(warp::path("login"))
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(login_handler);

    let register = api_prefix
        .and(warp::path("register"))
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(warp::header::optional::<String>("x-bootstrap-token"))
        .and(with_state(state.clone()))
        .and_then(register_handler);

    let refresh = api_prefix
        .and(warp::path("refresh"))
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(refresh_handler);

    let logout = api_prefix
        .and(warp::path("logout"))
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_state(state))
        .and_then(logout_handler);

    login.or(register).or(refresh).or(logout).boxed()
}

pub fn authenticated(
    state: AppState,
) -> impl Filter<Extract = (AuthenticatedUser,), Error = Rejection> + Clone {
    warp::header::optional::<String>("authorization")
        .and(with_state(state))
        .and_then(validate_access_token)
}

async fn validate_access_token(
    authorization: Option<String>,
    state: AppState,
) -> Result<AuthenticatedUser, Rejection> {
    let token = authorization
        .as_deref()
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| warp::reject::custom(AuthenticationRequired))?;

    let claims = decode_claims(token, "access", &state.config.jwt_secret)
        .map_err(|_| warp::reject::custom(AuthenticationRequired))?;

    let user_id =
        Uuid::parse_str(&claims.sub).map_err(|_| warp::reject::custom(AuthenticationRequired))?;
    let session_id =
        Uuid::parse_str(&claims.jti).map_err(|_| warp::reject::custom(AuthenticationRequired))?;
    let producer_id = Uuid::parse_str(&claims.producer_id)
        .map_err(|_| warp::reject::custom(AuthenticationRequired))?;
    let role = claims
        .role
        .parse::<crate::models::user::UserRole>()
        .map_err(|_| warp::reject::custom(AuthenticationRequired))?;

    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT u.*
        FROM users u
        INNER JOIN auth_sessions s ON s.user_id = u.id
        WHERE u.id = $1
          AND s.id = $2
          AND s.revoked_at IS NULL
          AND s.expires_at > NOW()
        "#,
    )
    .bind(user_id)
    .bind(session_id)
    .fetch_optional(&state.db.pool)
    .await
    .map_err(|error| {
        tracing::error!("Erreur de vérification de session: {:?}", error);
        warp::reject::custom(AuthenticationRequired)
    })?
    .filter(|user| user.is_active && user.producer_id == producer_id && user.role == claims.role)
    .ok_or_else(|| warp::reject::custom(AuthenticationRequired))?;

    Ok(AuthenticatedUser {
        producer_id: user.producer_id,
        role,
    })
}

fn with_state(state: AppState) -> impl Filter<Extract = (AppState,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn login_handler(request: LoginRequest, state: AppState) -> Result<impl Reply, Rejection> {
    let user = match sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&request.email)
        .fetch_optional(&state.db.pool)
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Ok(auth_error(
                "Identifiants invalides",
                warp::http::StatusCode::UNAUTHORIZED,
            ))
        }
        Err(error) => {
            return Ok(error_response(
                "Erreur lors de la lecture de l'utilisateur",
                error,
            ))
        }
    };

    if !user.is_active {
        return Ok(auth_error(
            "Utilisateur désactivé",
            warp::http::StatusCode::FORBIDDEN,
        ));
    }

    if let Err(error) = verify_password(&request.password, &user.password_hash) {
        tracing::warn!("Échec login pour {}: {}", request.email, error);
        return Ok(auth_error(
            "Identifiants invalides",
            warp::http::StatusCode::UNAUTHORIZED,
        ));
    }

    if user.totp_enabled && request.totp_code.as_deref().unwrap_or("").trim().is_empty() {
        return Ok(auth_error(
            "Code TOTP requis pour cet utilisateur",
            warp::http::StatusCode::UNAUTHORIZED,
        ));
    }

    if let Err(error) =
        sqlx::query("UPDATE users SET last_login = NOW(), updated_at = NOW() WHERE id = $1")
            .bind(user.id)
            .execute(&state.db.pool)
            .await
    {
        return Ok(error_response(
            "Erreur lors de la mise à jour de la dernière connexion",
            error,
        ));
    }

    let response = match issue_login_response(&user, &state).await {
        Ok(response) => response,
        Err(error) => {
            return Ok(error_response(
                "Erreur lors de la génération des jetons",
                error,
            ))
        }
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&response),
        warp::http::StatusCode::OK,
    ))
}

async fn register_handler(
    request: RegisterRequest,
    bootstrap_token: Option<String>,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    if !bootstrap_token_matches(
        state.config.bootstrap_token.as_deref(),
        bootstrap_token.as_deref(),
    ) {
        return Ok(auth_error(
            "Initialisation non autorisée",
            warp::http::StatusCode::FORBIDDEN,
        ));
    }

    if request.password.len() < state.config.security.password_min_length {
        return Ok(auth_error(
            &format!(
                "Le mot de passe doit faire au moins {} caractères",
                state.config.security.password_min_length
            ),
            warp::http::StatusCode::BAD_REQUEST,
        ));
    }

    let password_hash = match hash_password(&request.password) {
        Ok(hash) => hash,
        Err(error) => return Ok(error_response("Erreur lors du hash du mot de passe", error)),
    };

    let mut transaction = match state.db.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return Ok(error_response("Erreur lors de l'initialisation", error)),
    };

    if let Err(error) = sqlx::query("LOCK TABLE users IN SHARE ROW EXCLUSIVE MODE")
        .execute(&mut *transaction)
        .await
    {
        return Ok(error_response("Erreur lors de l'initialisation", error));
    }

    match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&mut *transaction)
        .await
    {
        Ok(0) => {}
        Ok(_) => {
            return Ok(auth_error(
                "Initialisation déjà effectuée",
                warp::http::StatusCode::FORBIDDEN,
            ))
        }
        Err(error) => return Ok(error_response("Erreur lors de l'initialisation", error)),
    }

    let producer_id = match resolve_bootstrap_producer_id(
        request.producer_id,
        request.producer.as_ref(),
        &request.email,
        &mut transaction,
    )
    .await
    {
        Ok(producer_id) => producer_id,
        Err(response) => return Ok(response),
    };

    let user_id = Uuid::new_v4();
    let user = match sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (
            id, producer_id, email, password_hash, first_name, last_name, role, is_active, totp_enabled
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, true, false)
        RETURNING *
        "#,
    )
    .bind(user_id)
    .bind(producer_id)
    .bind(&request.email)
    .bind(password_hash)
    .bind(&request.first_name)
    .bind(&request.last_name)
    .bind("admin")
    .fetch_one(&mut *transaction)
    .await
    {
        Ok(user) => user,
        Err(error) => return Ok(error_response("Erreur lors de la création utilisateur", error)),
    };

    if let Err(error) = transaction.commit().await {
        return Ok(error_response("Erreur lors de l'initialisation", error));
    }

    let response = match issue_login_response(&user, &state).await {
        Ok(response) => response,
        Err(error) => {
            return Ok(error_response(
                "Erreur lors de la génération des jetons",
                error,
            ))
        }
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&response),
        warp::http::StatusCode::CREATED,
    ))
}

async fn refresh_handler(
    request: RefreshRequest,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    let claims = match decode_token(&request.refresh_token, "refresh", &state) {
        Ok(claims) => claims,
        Err(response) => return Ok(response),
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(user_id) => user_id,
        Err(_) => {
            return Ok(auth_error(
                "Token invalide",
                warp::http::StatusCode::UNAUTHORIZED,
            ))
        }
    };

    let session_id = match Uuid::parse_str(&claims.jti) {
        Ok(session_id) => session_id,
        Err(_) => {
            return Ok(auth_error(
                "Token invalide",
                warp::http::StatusCode::UNAUTHORIZED,
            ))
        }
    };

    let mut transaction = match state.db.pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => return Ok(error_response("Erreur lors du refresh token", error)),
    };

    let session = match sqlx::query_as::<_, AuthSession>(
        r#"
        SELECT user_id, family_id, expires_at, revoked_at
        FROM auth_sessions
        WHERE id = $1
        FOR UPDATE
        "#,
    )
    .bind(session_id)
    .fetch_optional(&mut *transaction)
    .await
    {
        Ok(Some(session)) => session,
        Ok(None) => return Ok(invalid_token()),
        Err(error) => return Ok(error_response("Erreur lors du refresh token", error)),
    };

    if session.user_id != user_id
        || session.revoked_at.is_some()
        || session.expires_at <= Utc::now()
    {
        if let Err(error) = revoke_session_family(session.family_id, &mut transaction).await {
            return Ok(error_response(
                "Erreur lors de la révocation de session",
                error,
            ));
        }
        if let Err(error) = transaction.commit().await {
            return Ok(error_response(
                "Erreur lors de la révocation de session",
                error,
            ));
        }
        return Ok(invalid_token());
    }

    let user = match sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&mut *transaction)
        .await
    {
        Ok(Some(user))
            if user.is_active
                && user.producer_id.to_string() == claims.producer_id
                && user.role == claims.role =>
        {
            user
        }
        Ok(_) => return Ok(invalid_token()),
        Err(error) => return Ok(error_response("Erreur lors du refresh token", error)),
    };

    let new_session_id = Uuid::new_v4();
    let response = match build_login_response(&user, &state, new_session_id) {
        Ok(response) => response,
        Err(error) => {
            return Ok(error_response(
                "Erreur lors de la génération des jetons",
                error,
            ))
        }
    };

    if let Err(error) = sqlx::query(
        r#"
        INSERT INTO auth_sessions (id, family_id, user_id, expires_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(new_session_id)
    .bind(session.family_id)
    .bind(user.id)
    .bind(refresh_expiry(&state))
    .execute(&mut *transaction)
    .await
    {
        return Ok(error_response(
            "Erreur lors de la rotation de session",
            error,
        ));
    }

    if let Err(error) = sqlx::query(
        r#"
        UPDATE auth_sessions
        SET revoked_at = NOW(), last_used_at = NOW(), replaced_by = $2
        WHERE id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(session_id)
    .bind(new_session_id)
    .execute(&mut *transaction)
    .await
    {
        return Ok(error_response(
            "Erreur lors de la rotation de session",
            error,
        ));
    }

    if let Err(error) = transaction.commit().await {
        return Ok(error_response(
            "Erreur lors de la rotation de session",
            error,
        ));
    }

    Ok(warp::reply::with_status(
        warp::reply::json(&response),
        warp::http::StatusCode::OK,
    ))
}

async fn logout_handler(request: RefreshRequest, state: AppState) -> Result<impl Reply, Rejection> {
    let claims = match decode_token(&request.refresh_token, "refresh", &state) {
        Ok(claims) => claims,
        Err(response) => return Ok(response),
    };

    let (session_id, user_id) = match (Uuid::parse_str(&claims.jti), Uuid::parse_str(&claims.sub)) {
        (Ok(session_id), Ok(user_id)) => (session_id, user_id),
        _ => return Ok(invalid_token()),
    };

    if let Err(error) = sqlx::query(
        r#"
        UPDATE auth_sessions
        SET revoked_at = COALESCE(revoked_at, NOW())
        WHERE family_id = (
            SELECT family_id FROM auth_sessions WHERE id = $1 AND user_id = $2
        )
        "#,
    )
    .bind(session_id)
    .bind(user_id)
    .execute(&state.db.pool)
    .await
    {
        return Ok(error_response("Erreur lors de la déconnexion", error));
    }

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "success": true,
            "message": "Déconnexion effectuée"
        })),
        warp::http::StatusCode::OK,
    ))
}

async fn issue_login_response(
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

fn build_login_response(
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

fn refresh_expiry(state: &AppState) -> DateTime<Utc> {
    Utc::now() + Duration::hours(refresh_validity_hours(state) as i64)
}

fn refresh_validity_hours(state: &AppState) -> u64 {
    state
        .config
        .jwt_expiration_hours
        .saturating_mul(24)
        .min(i64::MAX as u64)
}

async fn revoke_session_family(
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

fn decode_token(
    token: &str,
    expected_type: &str,
    state: &AppState,
) -> Result<TokenClaims, warp::reply::WithStatus<warp::reply::Json>> {
    decode_claims(token, expected_type, &state.config.jwt_secret)
        .map_err(|_| auth_error("Token invalide", warp::http::StatusCode::UNAUTHORIZED))
}

fn decode_claims(
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

fn bootstrap_token_matches(configured: Option<&str>, provided: Option<&str>) -> bool {
    let (Some(configured), Some(provided)) = (configured, provided) else {
        return false;
    };

    configured.len() >= 32
        && configured.len() == provided.len()
        && configured
            .bytes()
            .zip(provided.bytes())
            .fold(0_u8, |difference, (left, right)| {
                difference | (left ^ right)
            })
            == 0
}

fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

fn verify_password(
    password: &str,
    password_hash: &str,
) -> Result<(), argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(password_hash)?;
    Argon2::default().verify_password(password.as_bytes(), &parsed_hash)
}

async fn resolve_bootstrap_producer_id(
    requested_producer_id: Option<Uuid>,
    requested_producer: Option<&BootstrapProducerRequest>,
    user_email: &str,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<Uuid, warp::reply::WithStatus<warp::reply::Json>> {
    if let Some(producer_id) = requested_producer_id {
        match sqlx::query_scalar::<_, Uuid>("SELECT id FROM producers WHERE id = $1")
            .bind(producer_id)
            .fetch_optional(&mut **transaction)
            .await
        {
            Ok(Some(_)) => Ok(producer_id),
            Ok(None) => Err(auth_error(
                "Producteur introuvable",
                warp::http::StatusCode::BAD_REQUEST,
            )),
            Err(error) => Err(error_response(
                "Erreur lors de la vérification du producteur",
                error,
            )),
        }
    } else if let Some(producer_id) =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM producers ORDER BY created_at ASC LIMIT 1")
            .fetch_optional(&mut **transaction)
            .await
            .map_err(|error| error_response("Erreur lors de la recherche de producteur", error))?
    {
        Ok(producer_id)
    } else {
        let Some(producer) = requested_producer else {
            return Err(auth_error(
                "Les informations de l'exploitation sont requises pour la première inscription",
                warp::http::StatusCode::BAD_REQUEST,
            ));
        };

        if [
            producer.raison_sociale.as_str(),
            producer.adresse.as_str(),
            producer.code_postal.as_str(),
            producer.ville.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(auth_error(
                "Les informations de l'exploitation sont incomplètes",
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }

        let producer_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO producers (
                id, raison_sociale, adresse, code_postal, ville, pays, email
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(producer_id)
        .bind(producer.raison_sociale.trim())
        .bind(producer.adresse.trim())
        .bind(producer.code_postal.trim())
        .bind(producer.ville.trim())
        .bind(
            producer
                .pays
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("France"),
        )
        .bind(user_email.trim())
        .execute(&mut **transaction)
        .await
        .map_err(|error| error_response("Erreur lors de la création du producteur", error))?;

        Ok(producer_id)
    }
}

fn auth_error(
    message: &str,
    status: warp::http::StatusCode,
) -> warp::reply::WithStatus<warp::reply::Json> {
    warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "success": false,
            "error": message
        })),
        status,
    )
}

fn invalid_token() -> warp::reply::WithStatus<warp::reply::Json> {
    auth_error("Token invalide", warp::http::StatusCode::UNAUTHORIZED)
}

fn error_response<E: std::fmt::Display>(
    message: &str,
    error: E,
) -> warp::reply::WithStatus<warp::reply::Json> {
    tracing::error!("{}: {}", message, error);
    warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "success": false,
            "error": message
        })),
        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        bootstrap_token_matches, decode_claims, hash_password, verify_password, RegisterRequest,
        TokenClaims,
    };
    use jsonwebtoken::{encode, EncodingKey, Header};

    #[test]
    fn bootstrap_requires_a_long_exact_token() {
        let token = "a-very-long-bootstrap-token-123456789";
        assert!(bootstrap_token_matches(Some(token), Some(token)));
        assert!(!bootstrap_token_matches(Some(token), Some("wrong")));
        assert!(!bootstrap_token_matches(
            Some("too-short"),
            Some("too-short")
        ));
        assert!(!bootstrap_token_matches(None, Some(token)));
    }

    #[test]
    fn password_hash_roundtrip_works() {
        let hash = hash_password("secret123").expect("hash password");
        verify_password("secret123", &hash).expect("verify password");
        assert!(verify_password("bad", &hash).is_err());
    }

    #[test]
    fn registration_rejects_a_client_selected_role() {
        let request = serde_json::json!({
            "producer_id": null,
            "email": "admin@example.test",
            "password": "a-long-password",
            "first_name": "Ada",
            "last_name": "Lovelace",
            "role": "admin"
        });

        assert!(serde_json::from_value::<RegisterRequest>(request).is_err());
    }

    #[test]
    fn registration_accepts_first_producer_details() {
        let request = serde_json::json!({
            "producer_id": null,
            "producer": {
                "raison_sociale": "Ferme des Trois Chênes",
                "adresse": "12 chemin des Prés",
                "code_postal": "47000",
                "ville": "Agen",
                "pays": "France"
            },
            "email": "admin@example.test",
            "password": "a-long-password",
            "first_name": "Ada",
            "last_name": "Lovelace"
        });

        let request =
            serde_json::from_value::<RegisterRequest>(request).expect("deserialize registration");
        let producer = request.producer.expect("bootstrap producer details");
        assert_eq!(producer.raison_sociale, "Ferme des Trois Chênes");
        assert_eq!(producer.ville, "Agen");
    }

    #[test]
    fn access_decoder_rejects_a_refresh_token() {
        let session_id = uuid::Uuid::new_v4();
        let claims = TokenClaims {
            sub: uuid::Uuid::new_v4().to_string(),
            jti: session_id.to_string(),
            producer_id: uuid::Uuid::new_v4().to_string(),
            role: "admin".to_string(),
            token_type: "refresh".to_string(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            iat: chrono::Utc::now().timestamp() as usize,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(b"test-secret"),
        )
        .expect("encode test token");

        assert!(decode_claims(&token, "access", "test-secret").is_err());
        let decoded = decode_claims(&token, "refresh", "test-secret")
            .expect("decode refresh token with session id");
        assert_eq!(decoded.jti, session_id.to_string());
    }
}
