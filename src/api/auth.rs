use crate::api::AppState;
use crate::models::user::{LoginRequest, LoginResponse, User, UserProfile};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use uuid::Uuid;
use warp::{Filter, Rejection, Reply};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisterRequest {
    producer_id: Option<Uuid>,
    email: String,
    password: String,
    first_name: String,
    last_name: String,
}

#[derive(Debug, Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TokenClaims {
    sub: String,
    producer_id: String,
    role: String,
    token_type: String,
    exp: usize,
    iat: usize,
}

#[derive(Debug)]
pub(crate) struct AuthenticationRequired;

impl warp::reject::Reject for AuthenticationRequired {}

pub fn routes(state: AppState) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let api_prefix = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("auth"));

    let login = api_prefix
        .clone()
        .and(warp::path("login"))
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(login_handler);

    let register = api_prefix
        .clone()
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
        .and(with_state(state))
        .and_then(refresh_handler);

    login.or(register).or(refresh)
}

pub fn require_access_token(
    state: AppState,
) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::header::optional::<String>("authorization")
        .and(with_state(state))
        .and_then(validate_access_token)
        .untuple_one()
}

async fn validate_access_token(
    authorization: Option<String>,
    state: AppState,
) -> Result<(), Rejection> {
    let token = authorization
        .as_deref()
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| warp::reject::custom(AuthenticationRequired))?;

    let claims = decode_claims(token, "access", &state.config.jwt_secret)
        .map_err(|_| warp::reject::custom(AuthenticationRequired))?;

    Uuid::parse_str(&claims.sub)
        .and_then(|_| Uuid::parse_str(&claims.producer_id))
        .map_err(|_| warp::reject::custom(AuthenticationRequired))?;
    claims
        .role
        .parse::<crate::models::user::UserRole>()
        .map_err(|_| warp::reject::custom(AuthenticationRequired))?;

    Ok(())
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

    let response = match build_login_response(&user, &state) {
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

    let producer_id = match resolve_producer_id(request.producer_id, &state).await {
        Ok(producer_id) => producer_id,
        Err(response) => return Ok(response),
    };

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

    let response = match build_login_response(&user, &state) {
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

    let user = match sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&state.db.pool)
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Ok(auth_error(
                "Utilisateur introuvable",
                warp::http::StatusCode::UNAUTHORIZED,
            ))
        }
        Err(error) => return Ok(error_response("Erreur lors du refresh token", error)),
    };

    let response = match build_login_response(&user, &state) {
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

fn build_login_response(
    user: &User,
    state: &AppState,
) -> Result<LoginResponse, jsonwebtoken::errors::Error> {
    let access_token = encode_token(
        user,
        "access",
        state.config.jwt_expiration_hours,
        &state.config.jwt_secret,
    )?;
    let refresh_token = encode_token(
        user,
        "refresh",
        state.config.jwt_expiration_hours * 24,
        &state.config.jwt_secret,
    )?;

    Ok(LoginResponse {
        access_token,
        refresh_token,
        expires_in: (state.config.jwt_expiration_hours * 3600) as i64,
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
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let claims = TokenClaims {
        sub: user.id.to_string(),
        producer_id: user.producer_id.to_string(),
        role: user.role.clone(),
        token_type: token_type.to_string(),
        iat: now.timestamp() as usize,
        exp: (now + Duration::hours(validity_hours as i64)).timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
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

async fn resolve_producer_id(
    requested_producer_id: Option<Uuid>,
    state: &AppState,
) -> Result<Uuid, warp::reply::WithStatus<warp::reply::Json>> {
    if let Some(producer_id) = requested_producer_id {
        match sqlx::query_scalar::<_, Uuid>("SELECT id FROM producers WHERE id = $1")
            .bind(producer_id)
            .fetch_optional(&state.db.pool)
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
    } else {
        match sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM producers ORDER BY created_at ASC LIMIT 1",
        )
        .fetch_optional(&state.db.pool)
        .await
        {
            Ok(Some(producer_id)) => Ok(producer_id),
            Ok(None) => Err(auth_error(
                "Aucun producteur disponible",
                warp::http::StatusCode::BAD_REQUEST,
            )),
            Err(error) => Err(error_response(
                "Erreur lors de la recherche de producteur",
                error,
            )),
        }
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
    fn access_decoder_rejects_a_refresh_token() {
        let claims = TokenClaims {
            sub: uuid::Uuid::new_v4().to_string(),
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
    }
}
