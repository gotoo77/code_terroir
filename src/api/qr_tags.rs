use crate::api::auth::AuthenticatedUser;
use crate::api::authorization::{is_allowed, Action};
use crate::api::{auth, AppState};
use crate::models::qr_tag::{
    generate_unique_slug, CreateQRTagRequest, QRFormat, QRTag, RecordScanRequest,
};
use sqlx::types::ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use sqlx::FromRow;
use std::convert::Infallible;
use uuid::Uuid;
use warp::{Filter, Rejection, Reply};
// use base64::{Engine as _, engine::general_purpose}; // Pas utilisé pour l'instant

#[derive(Debug, FromRow)]
struct QRAnalyticsRow {
    total_scans: Option<i64>,
    unique_scans: Option<i64>,
    scans_today: Option<i64>,
    scans_this_week: Option<i64>,
    scans_this_month: Option<i64>,
}

pub fn routes(state: AppState) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let api_prefix = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("qr"));

    // List QR tags
    let list_qr_tags = api_prefix
        .and(warp::get())
        .and(warp::path::end())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(list_qr_tags_handler);

    // Get single QR tag
    let get_qr_tag = api_prefix
        .and(warp::get())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(get_qr_tag_handler);

    // Create QR tag
    let create_qr_tag = api_prefix
        .and(warp::post())
        .and(warp::path::end())
        .and(crate::api::json_body(state.clone()))
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(create_qr_tag_handler);

    // Scan QR code by slug
    let scan_qr = api_prefix
        .and(warp::post())
        .and(warp::path("scan"))
        .and(warp::path::param::<String>()) // slug
        .and(warp::path::end())
        .and(crate::api::json_body(state.clone()))
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(scan_qr_handler);

    // Get QR analytics
    let get_qr_analytics = api_prefix
        .and(warp::get())
        .and(warp::path::param::<String>()) // qr_tag_id
        .and(warp::path("analytics"))
        .and(warp::path::end())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(get_qr_analytics_handler);

    list_qr_tags
        .or(get_qr_tag)
        .or(create_qr_tag)
        .or(scan_qr)
        .or(get_qr_analytics)
}

// Helper pour extraire l'état de l'application
fn with_state(state: AppState) -> impl Filter<Extract = (AppState,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

// Handler pour lister tous les QR tags
async fn list_qr_tags_handler(
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    match sqlx::query_as::<_, QRTag>(
        r#"
        SELECT q.*
        FROM qr_tags q
        INNER JOIN batches b ON b.id = q.batch_id
        WHERE b.producer_id = $1
        ORDER BY q.created_at DESC
        "#,
    )
    .bind(authenticated.producer_id)
    .fetch_all(&state.db.pool)
    .await
    {
        Ok(qr_tags) => {
            let response = serde_json::json!({
                "success": true,
                "count": qr_tags.len(),
                "qr_tags": qr_tags
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            ))
        }
        Err(e) => {
            tracing::error!("Erreur lors de la récupération des QR tags: {:?}", e);
            let response = serde_json::json!({
                "success": false,
                "error": "Erreur lors de la récupération des QR tags",
                "details": "Erreur interne"
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

// Handler pour récupérer un QR tag par ID
async fn get_qr_tag_handler(
    id: String,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    // Valider que l'ID est un UUID valide
    let qr_tag_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            let response = serde_json::json!({
                "success": false,
                "error": "ID QR tag invalide",
                "details": "L'ID doit être un UUID valide"
            });
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    match sqlx::query_as::<_, QRTag>(
        r#"
        SELECT q.*
        FROM qr_tags q
        INNER JOIN batches b ON b.id = q.batch_id
        WHERE q.id = $1 AND b.producer_id = $2
        "#,
    )
    .bind(qr_tag_id)
    .bind(authenticated.producer_id)
    .fetch_optional(&state.db.pool)
    .await
    {
        Ok(Some(qr_tag)) => {
            let response = serde_json::json!({
                "success": true,
                "qr_tag": qr_tag
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            ))
        }
        Ok(None) => {
            let response = serde_json::json!({
                "success": false,
                "error": "QR tag non trouvé",
                "qr_tag_id": id
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::NOT_FOUND,
            ))
        }
        Err(e) => {
            tracing::error!("Erreur lors de la récupération du QR tag {}: {:?}", id, e);
            let response = serde_json::json!({
                "success": false,
                "error": "Erreur lors de la récupération du QR tag",
                "details": "Erreur interne"
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

// Handler pour créer un nouveau QR tag
async fn create_qr_tag_handler(
    create_request: CreateQRTagRequest,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    if !is_allowed(&authenticated.role, Action::ManageQrTags) {
        return Ok(forbidden());
    }

    // Générer un nouvel ID
    let qr_tag_id = Uuid::new_v4();

    // Vérifier que le batch_id existe
    match sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM batches WHERE id = $1 AND producer_id = $2)",
    )
    .bind(create_request.batch_id)
    .bind(authenticated.producer_id)
    .fetch_one(&state.db.pool)
    .await
    {
        Ok(false) => {
            let response = serde_json::json!({
                "success": false,
                "error": "Lot de production non trouvé",
                "details": format!("Aucun lot trouvé avec l'ID: {}", create_request.batch_id)
            });
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
        Err(e) => {
            tracing::error!("Erreur lors de la vérification du batch: {:?}", e);
            let response = serde_json::json!({
                "success": false,
                "error": "Erreur lors de la vérification du lot",
                "details": "Erreur interne"
            });
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
        Ok(true) => {} // Le batch existe, on continue
    }

    // Générer un slug unique
    let slug = generate_unique_slug();
    let short_url = format!("{}/t/{}", state.config.qr_base_url, slug);

    // Générer les QR codes selon le format demandé
    let format = create_request.format.unwrap_or_default();
    let (qr_svg, qr_png) = generate_qr_codes(&short_url, &format);

    // Insérer le QR tag dans la base de données
    match sqlx::query_as::<_, QRTag>(
        r#"
        INSERT INTO qr_tags (
            id, batch_id, slug, short_url, qr_code_svg, qr_code_png,
            print_count, scan_count, is_active, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, 0, 0, true, NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(qr_tag_id)
    .bind(create_request.batch_id)
    .bind(&slug)
    .bind(&short_url)
    .bind(&qr_svg)
    .bind(&qr_png)
    .fetch_one(&state.db.pool)
    .await
    {
        Ok(qr_tag) => {
            tracing::info!("QR tag créé avec succès: {} ({})", qr_tag.slug, qr_tag.id);
            let response = serde_json::json!({
                "success": true,
                "message": "QR tag créé avec succès",
                "qr_tag": qr_tag
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::CREATED,
            ))
        }
        Err(e) => {
            tracing::error!("Erreur lors de la création du QR tag: {:?}", e);
            let response = serde_json::json!({
                "success": false,
                "error": "Erreur lors de la création du QR tag",
                "details": "Erreur interne"
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

// Handler pour enregistrer un scan de QR code
async fn scan_qr_handler(
    slug: String,
    scan_request: RecordScanRequest,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    // Récupérer le QR tag par son slug
    let qr_tag = match sqlx::query_as::<_, QRTag>(
        r#"
        SELECT q.*
        FROM qr_tags q
        INNER JOIN batches b ON b.id = q.batch_id
        WHERE q.slug = $1 AND q.is_active = true AND b.producer_id = $2
        "#,
    )
    .bind(&slug)
    .bind(authenticated.producer_id)
    .fetch_optional(&state.db.pool)
    .await
    {
        Ok(Some(qr_tag)) => qr_tag,
        Ok(None) => {
            let response = serde_json::json!({
                "success": false,
                "error": "QR code non trouvé ou inactif",
                "slug": slug
            });
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::NOT_FOUND,
            ));
        }
        Err(e) => {
            tracing::error!("Erreur lors de la recherche du QR tag {}: {:?}", slug, e);
            let response = serde_json::json!({
                "success": false,
                "error": "Erreur lors de la recherche du QR code",
                "details": "Erreur interne"
            });
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    // Enregistrer le scan
    let scan_id = Uuid::new_v4();

    // Convertir l'adresse IP en type INET pour PostgreSQL
    let ip_addr = parse_ip_network(scan_request.ip_address.as_deref());

    match sqlx::query(
        r#"
        INSERT INTO qr_scans (
            id, qr_tag_id, batch_id, ip_address, user_agent, referer, scanned_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, NOW())
        "#,
    )
    .bind(scan_id)
    .bind(qr_tag.id)
    .bind(qr_tag.batch_id)
    .bind(ip_addr)
    .bind(&scan_request.user_agent)
    .bind(&scan_request.referer)
    .execute(&state.db.pool)
    .await
    {
        Ok(_) => {
            // Incrémenter le compteur de scans
            let _ = sqlx::query(
                r#"
                UPDATE qr_tags q
                SET scan_count = scan_count + 1, last_scanned_at = NOW()
                WHERE q.id = $1
                  AND EXISTS (
                      SELECT 1
                      FROM batches b
                      WHERE b.id = q.batch_id
                        AND b.producer_id = $2
                  )
                "#,
            )
            .bind(qr_tag.id)
            .bind(authenticated.producer_id)
            .execute(&state.db.pool)
            .await;

            tracing::info!(
                "Scan enregistré pour QR tag: {} ({})",
                qr_tag.slug,
                qr_tag.id
            );
            let response = serde_json::json!({
                "success": true,
                "message": "Scan enregistré avec succès",
                "qr_tag_id": qr_tag.id,
                "batch_id": qr_tag.batch_id,
                "scan_id": scan_id
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            ))
        }
        Err(e) => {
            tracing::error!("Erreur lors de l'enregistrement du scan: {:?}", e);
            let response = serde_json::json!({
                "success": false,
                "error": "Erreur lors de l'enregistrement du scan",
                "details": "Erreur interne"
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

// Handler pour récupérer les analytics d'un QR tag
async fn get_qr_analytics_handler(
    id: String,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    let qr_tag_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            let response = serde_json::json!({
                "success": false,
                "error": "ID QR tag invalide",
                "details": "L'ID doit être un UUID valide"
            });
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    // Récupérer les statistiques de base
    let stats = match sqlx::query_as::<_, QRAnalyticsRow>(
        r#"
        SELECT
            COUNT(s.id) as total_scans,
            COUNT(DISTINCT s.ip_address) as unique_scans,
            COUNT(CASE WHEN s.scanned_at::date = CURRENT_DATE THEN 1 END) as scans_today,
            COUNT(CASE WHEN s.scanned_at >= CURRENT_DATE - INTERVAL '7 days' THEN 1 END) as scans_this_week,
            COUNT(CASE WHEN s.scanned_at >= CURRENT_DATE - INTERVAL '30 days' THEN 1 END) as scans_this_month
        FROM qr_tags q
        INNER JOIN batches b ON b.id = q.batch_id
        LEFT JOIN qr_scans s ON s.qr_tag_id = q.id
        WHERE q.id = $1 AND b.producer_id = $2
        GROUP BY q.id
        "#,
    )
    .bind(qr_tag_id)
    .bind(authenticated.producer_id)
    .fetch_optional(&state.db.pool)
    .await
    {
        Ok(Some(stats)) => stats,
        Ok(None) => {
            let response = serde_json::json!({
                "success": false,
                "error": "QR tag non trouvé"
            });
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::NOT_FOUND,
            ));
        }
        Err(e) => {
            tracing::error!("Erreur lors de la récupération des statistiques: {:?}", e);
            let response = serde_json::json!({
                "success": false,
                "error": "Erreur lors de la récupération des statistiques",
                "details": "Erreur interne"
            });
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    let response = serde_json::json!({
        "success": true,
        "analytics": {
            "qr_tag_id": qr_tag_id,
            "total_scans": stats.total_scans.unwrap_or(0),
            "unique_scans": stats.unique_scans.unwrap_or(0),
            "scans_today": stats.scans_today.unwrap_or(0),
            "scans_this_week": stats.scans_this_week.unwrap_or(0),
            "scans_this_month": stats.scans_this_month.unwrap_or(0)
        }
    });

    Ok(warp::reply::with_status(
        warp::reply::json(&response),
        warp::http::StatusCode::OK,
    ))
}

fn forbidden() -> warp::reply::WithStatus<warp::reply::Json> {
    warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "success": false,
            "error": "Action non autorisée"
        })),
        warp::http::StatusCode::FORBIDDEN,
    )
}

fn parse_ip_network(ip: Option<&str>) -> Option<IpNetwork> {
    let ip = ip?;

    match ip.parse::<IpNetwork>() {
        Ok(network) => Some(network),
        Err(_) => match ip.parse::<std::net::IpAddr>() {
            Ok(std::net::IpAddr::V4(ipv4)) => Ipv4Network::new(ipv4, 32).ok().map(IpNetwork::V4),
            Ok(std::net::IpAddr::V6(ipv6)) => Ipv6Network::new(ipv6, 128).ok().map(IpNetwork::V6),
            Err(_) => {
                tracing::warn!("Adresse IP invalide reçue: {}", ip);
                None
            }
        },
    }
}

// Fonction utilitaire pour générer les codes QR
fn generate_qr_codes(url: &str, format: &QRFormat) -> (Option<String>, Option<Vec<u8>>) {
    use qrcode::render::svg;
    use qrcode::QrCode;

    // Créer le QR code à partir de l'URL
    let qr_code = match QrCode::new(url) {
        Ok(code) => code,
        Err(e) => {
            tracing::error!("Erreur lors de la génération du QR code: {:?}", e);
            // En cas d'erreur, retourner des placeholders
            return generate_qr_placeholders(url, format);
        }
    };

    let svg_code = match format {
        QRFormat::Svg | QRFormat::Both => {
            let svg_string = qr_code
                .render::<svg::Color>()
                .min_dimensions(200, 200)
                .dark_color(svg::Color("#000000"))
                .light_color(svg::Color("#FFFFFF"))
                .build();
            Some(svg_string)
        }
        QRFormat::Png => None,
    };

    let png_bytes = match format {
        QRFormat::Png | QRFormat::Both => {
            // Pour le PNG, on génère une représentation simple en bitmap
            let bitmap = qr_code
                .render::<qrcode::render::unicode::Dense1x2>()
                .dark_color(qrcode::render::unicode::Dense1x2::Light)
                .light_color(qrcode::render::unicode::Dense1x2::Dark)
                .build();

            // Pour une vraie implémentation PNG, il faudrait utiliser une librairie image
            // Pour maintenant, on stocke la représentation en tant que bytes UTF-8
            Some(bitmap.into_bytes())
        }
        QRFormat::Svg => None,
    };

    (svg_code, png_bytes)
}

// Fonction de fallback pour générer des placeholders en cas d'erreur
fn generate_qr_placeholders(url: &str, format: &QRFormat) -> (Option<String>, Option<Vec<u8>>) {
    let svg_placeholder = match format {
        QRFormat::Svg | QRFormat::Both => Some(format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="0 0 200 200">
                    <rect width="200" height="200" fill="white"/>
                    <rect x="10" y="10" width="180" height="180" fill="black"/>
                    <rect x="20" y="20" width="160" height="160" fill="white"/>
                    <text x="100" y="80" text-anchor="middle" font-family="Arial" font-size="8" fill="black">
                        QR CODE ERROR
                    </text>
                    <text x="100" y="100" text-anchor="middle" font-family="Arial" font-size="7" fill="black">
                        {}
                    </text>
                    <text x="100" y="120" text-anchor="middle" font-family="Arial" font-size="6" fill="black">
                        (Fallback placeholder)
                    </text>
                </svg>"#,
            url
        )),
        QRFormat::Png => None,
    };

    let png_placeholder = match format {
        QRFormat::Png | QRFormat::Both => {
            Some(vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) // PNG header
        }
        QRFormat::Svg => None,
    };

    (svg_placeholder, png_placeholder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ip_network_accepts_ipv4_and_ipv6_addresses() {
        assert!(matches!(
            parse_ip_network(Some("192.168.1.10")),
            Some(IpNetwork::V4(_))
        ));
        assert!(matches!(
            parse_ip_network(Some("2001:db8::1")),
            Some(IpNetwork::V6(_))
        ));
    }

    #[test]
    fn parse_ip_network_accepts_cidr_notation() {
        let network = parse_ip_network(Some("10.0.0.0/24"));
        assert!(matches!(network, Some(IpNetwork::V4(_))));
    }

    #[test]
    fn parse_ip_network_rejects_invalid_values() {
        assert_eq!(parse_ip_network(None), None);
        assert_eq!(parse_ip_network(Some("not-an-ip")), None);
    }

    #[test]
    fn generate_qr_placeholders_respects_requested_format() {
        let (svg, png) = generate_qr_placeholders("https://ct.local/t/demo", &QRFormat::Both);
        assert!(svg
            .as_ref()
            .is_some_and(|value| value.contains("QR CODE ERROR")));
        assert_eq!(
            png,
            Some(vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
        );

        let (svg_only, png_only) =
            generate_qr_placeholders("https://ct.local/t/demo", &QRFormat::Svg);
        assert!(svg_only.is_some());
        assert!(png_only.is_none());
    }

    #[test]
    fn qr_format_keeps_the_existing_json_contract() {
        assert!(matches!(
            serde_json::from_str::<QRFormat>("\"SVG\"").expect("deserialize SVG"),
            QRFormat::Svg
        ));
        assert!(matches!(
            serde_json::from_str::<QRFormat>("\"PNG\"").expect("deserialize PNG"),
            QRFormat::Png
        ));
        assert_eq!(
            serde_json::to_string(&QRFormat::Svg).expect("serialize SVG"),
            "\"SVG\""
        );
    }
}
