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
struct RegisterRequest {
    producer_id: Option<Uuid>,
    email: String,
    password: String,
    first_name: String,
    last_name: String,
    role: String,
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
    state: AppState,
) -> Result<impl Reply, Rejection> {
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

    match sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&request.email)
        .fetch_optional(&state.db.pool)
        .await
    {
        Ok(Some(_)) => {
            return Ok(auth_error(
                "Un utilisateur existe déjà avec cet email",
                warp::http::StatusCode::CONFLICT,
            ))
        }
        Ok(None) => {}
        Err(error) => {
            return Ok(error_response(
                "Erreur lors de la vérification email",
                error,
            ))
        }
    }

    let password_hash = match hash_password(&request.password) {
        Ok(hash) => hash,
        Err(error) => return Ok(error_response("Erreur lors du hash du mot de passe", error)),
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
    .bind(normalize_role(&request.role))
    .fetch_one(&state.db.pool)
    .await
    {
        Ok(user) => user,
        Err(error) => return Ok(error_response("Erreur lors de la création utilisateur", error)),
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
    let decoded = decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| auth_error("Token invalide", warp::http::StatusCode::UNAUTHORIZED))?;

    if decoded.claims.token_type != expected_type {
        return Err(auth_error(
            "Type de token invalide",
            warp::http::StatusCode::UNAUTHORIZED,
        ));
    }

    Ok(decoded.claims)
}
fn normalize_role(role: &str) -> String {
    match role.trim().to_ascii_lowercase().as_str() {
        "admin" | "quality" | "atelier" | "logistics" | "readonly" => {
            role.trim().to_ascii_lowercase()
        }
        _ => String::from("atelier"),
    }
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
            "error": message,
            "details": error.to_string()
        })),
        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
    )
}

#[cfg(test)]
mod tests {
    use super::{hash_password, normalize_role, verify_password};

    #[test]
    fn normalize_role_falls_back_to_atelier() {
        assert_eq!(normalize_role("ADMIN"), "admin");
        assert_eq!(normalize_role("nimportequoi"), "atelier");
    }

    #[test]
    fn password_hash_roundtrip_works() {
        let hash = hash_password("secret123").expect("hash password");
        verify_password("secret123", &hash).expect("verify password");
        assert!(verify_password("bad", &hash).is_err());
    }
}
