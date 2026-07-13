use crate::api::AppState;
use crate::models::batch::Batch;
use crate::models::product::Product;
use crate::models::qr_tag::QRTag;
use serde::{Deserialize, Serialize};
use sqlx::types::ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use sqlx::FromRow;
use std::convert::Infallible;
use uuid::Uuid;
use warp::{Filter, Rejection, Reply};

#[derive(Debug, Serialize)]
pub struct TraceabilityInfo {
    pub success: bool,
    pub message: String,
    pub qr_tag: Option<QRTag>,
    pub batch: Option<Batch>,
    pub product: Option<Product>,
    pub product_ingredients: Vec<PublicIngredientLine>,
    pub linked_producers: Vec<PublicProducerInfo>,
    pub scan_recorded: bool,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct PublicIngredientLine {
    pub ingredient_id: Uuid,
    pub ingredient_name: String,
    pub ingredient_category: Option<String>,
    pub ingredient_allergens: Vec<String>,
    pub producer_id: Uuid,
    pub producer_name: String,
    pub producer_city: String,
    pub producer_email: String,
    pub producer_phone: Option<String>,
    pub quantity: Option<f32>,
    pub unit: Option<String>,
    pub notes: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicProducerInfo {
    pub producer_id: Uuid,
    pub producer_name: String,
    pub producer_city: String,
    pub producer_email: String,
    pub producer_phone: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RecordVisitRequest {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
}

pub fn routes(state: AppState) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // Route principale pour récupérer les informations de traçabilité
    let get_traceability_info = warp::path("t")
        .and(warp::get())
        .and(warp::path::param::<String>()) // slug
        .and(warp::path::end())
        .and(with_state(state.clone()))
        .and_then(get_traceability_handler);

    // Route API pour récupérer les données JSON (pour les apps mobiles)
    let get_api_traceability = warp::path("api")
        .and(warp::path("trace"))
        .and(warp::get())
        .and(warp::path::param::<String>()) // slug
        .and(warp::path::end())
        .and(with_state(state.clone()))
        .and_then(get_api_traceability_handler);

    // Route pour enregistrer une visite (optionnel, pour analytics)
    let record_visit = warp::path("api")
        .and(warp::path("trace"))
        .and(warp::post())
        .and(warp::path::param::<String>()) // slug
        .and(warp::path("visit"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(record_visit_handler);

    get_traceability_info
        .or(get_api_traceability)
        .or(record_visit)
}

// Helper pour extraire l'état de l'application
fn with_state(state: AppState) -> impl Filter<Extract = (AppState,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

// Handler principal pour afficher la page de traçabilité
async fn get_traceability_handler(slug: String, state: AppState) -> Result<impl Reply, Rejection> {
    match get_traceability_data(&slug, &state).await {
        Ok(info) => {
            if info.success {
                // Générer une page HTML simple avec les informations
                let html = generate_traceability_html(&info);
                Ok(warp::reply::with_header(
                    warp::reply::html(html),
                    "Content-Type",
                    "text/html; charset=utf-8",
                ))
            } else {
                Ok(warp::reply::with_header(
                    warp::reply::html(generate_error_html(&slug)),
                    "Content-Type",
                    "text/html; charset=utf-8",
                ))
            }
        }
        Err(_) => Ok(warp::reply::with_header(
            warp::reply::html(generate_error_html(&slug)),
            "Content-Type",
            "text/html; charset=utf-8",
        )),
    }
}

// Handler API pour récupérer les données JSON
async fn get_api_traceability_handler(
    slug: String,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    match get_traceability_data(&slug, &state).await {
        Ok(info) => Ok(warp::reply::with_status(
            warp::reply::json(&info),
            if info.success {
                warp::http::StatusCode::OK
            } else {
                warp::http::StatusCode::NOT_FOUND
            },
        )),
        Err(e) => {
            let error_response = serde_json::json!({
                "success": false,
                "error": "Erreur lors de la récupération des données",
                "details": e.to_string()
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&error_response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

// Handler pour enregistrer une visite
async fn record_visit_handler(
    slug: String,
    visit_request: RecordVisitRequest,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    match get_traceability_data(&slug, &state).await {
        Ok(info) => {
            if let Some(qr_tag) = info.qr_tag.as_ref() {
                let scan_id = Uuid::new_v4();
                let ip_address = parse_ip_network(visit_request.ip_address.as_deref());

                if let Err(error) = sqlx::query(
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
                .bind(ip_address)
                .bind(visit_request.user_agent)
                .bind(visit_request.referer)
                .execute(&state.db.pool)
                .await
                {
                    let response = serde_json::json!({
                        "success": false,
                        "error": "Erreur lors de l'enregistrement de la visite",
                        "details": error.to_string()
                    });
                    return Ok(warp::reply::with_status(
                        warp::reply::json(&response),
                        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                    ));
                }

                let response = serde_json::json!({
                    "success": true,
                    "message": "Visite enregistrée",
                    "scan_id": scan_id,
                    "qr_tag_id": qr_tag.id,
                    "batch_id": info.batch.as_ref().map(|b| b.id)
                });
                Ok(warp::reply::with_status(
                    warp::reply::json(&response),
                    warp::http::StatusCode::OK,
                ))
            } else {
                let response = serde_json::json!({
                    "success": false,
                    "error": "QR code non trouvé"
                });
                Ok(warp::reply::with_status(
                    warp::reply::json(&response),
                    warp::http::StatusCode::NOT_FOUND,
                ))
            }
        }
        Err(e) => {
            let response = serde_json::json!({
                "success": false,
                "error": "Erreur lors de l'enregistrement de la visite",
                "details": e.to_string()
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

fn parse_ip_network(ip: Option<&str>) -> Option<IpNetwork> {
    let ip = ip?;

    match ip.parse::<IpNetwork>() {
        Ok(network) => Some(network),
        Err(_) => match ip.parse::<std::net::IpAddr>() {
            Ok(std::net::IpAddr::V4(ipv4)) => Ipv4Network::new(ipv4, 32).ok().map(IpNetwork::V4),
            Ok(std::net::IpAddr::V6(ipv6)) => Ipv6Network::new(ipv6, 128).ok().map(IpNetwork::V6),
            Err(_) => {
                tracing::warn!("Adresse IP invalide reçue sur la route publique: {}", ip);
                None
            }
        },
    }
}

// Fonction utilitaire pour récupérer toutes les données de traçabilité
async fn get_traceability_data(
    slug: &str,
    state: &AppState,
) -> Result<TraceabilityInfo, sqlx::Error> {
    // 1. Récupérer le QR tag
    let qr_tag =
        sqlx::query_as::<_, QRTag>("SELECT * FROM qr_tags WHERE slug = $1 AND is_active = true")
            .bind(slug)
            .fetch_optional(&state.db.pool)
            .await?;

    if let Some(qr_tag) = qr_tag {
        // 2. Récupérer le batch
        let batch = sqlx::query_as::<_, Batch>("SELECT * FROM batches WHERE id = $1")
            .bind(&qr_tag.batch_id)
            .fetch_optional(&state.db.pool)
            .await?;

        // 3. Récupérer le produit si le batch existe
        let product = if let Some(ref batch) = batch {
            sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
                .bind(&batch.product_id)
                .fetch_optional(&state.db.pool)
                .await?
        } else {
            None
        };

        let product_ingredients = if let Some(ref product) = product {
            load_public_ingredient_lines(product.id, state).await?
        } else {
            Vec::new()
        };
        let linked_producers = dedupe_public_producers(&product_ingredients);

        Ok(TraceabilityInfo {
            success: true,
            message: "Informations de traçabilité trouvées".to_string(),
            qr_tag: Some(qr_tag),
            batch,
            product,
            product_ingredients,
            linked_producers,
            scan_recorded: false,
        })
    } else {
        Ok(TraceabilityInfo {
            success: false,
            message: format!("Aucune information trouvée pour le code {}", slug),
            qr_tag: None,
            batch: None,
            product: None,
            product_ingredients: Vec::new(),
            linked_producers: Vec::new(),
            scan_recorded: false,
        })
    }
}

async fn load_public_ingredient_lines(
    product_id: Uuid,
    state: &AppState,
) -> Result<Vec<PublicIngredientLine>, sqlx::Error> {
    sqlx::query_as::<_, PublicIngredientLine>(
        r#"
        SELECT
            i.id AS ingredient_id,
            i.name AS ingredient_name,
            i.category AS ingredient_category,
            i.allergens AS ingredient_allergens,
            p.id AS producer_id,
            p.raison_sociale AS producer_name,
            p.ville AS producer_city,
            p.email AS producer_email,
            p.telephone AS producer_phone,
            pi.quantity,
            pi.unit,
            pi.notes,
            pi.sort_order
        FROM product_ingredients pi
        INNER JOIN ingredients i ON i.id = pi.ingredient_id
        INNER JOIN producers p ON p.id = pi.producer_id
        WHERE pi.product_id = $1
        ORDER BY pi.sort_order, i.name
        "#,
    )
    .bind(product_id)
    .fetch_all(&state.db.pool)
    .await
}

fn dedupe_public_producers(lines: &[PublicIngredientLine]) -> Vec<PublicProducerInfo> {
    let mut seen = std::collections::BTreeMap::new();
    for line in lines {
        seen.entry(line.producer_id)
            .or_insert_with(|| PublicProducerInfo {
                producer_id: line.producer_id,
                producer_name: line.producer_name.clone(),
                producer_city: line.producer_city.clone(),
                producer_email: line.producer_email.clone(),
                producer_phone: line.producer_phone.clone(),
            });
    }
    seen.into_values().collect()
}

fn compute_macro_distribution(product: &Product) -> Option<Vec<(String, String)>> {
    let total = product.glucides_100g? + product.lipides_100g? + product.proteines_100g?;
    if total <= 0.0 {
        return None;
    }

    Some(vec![
        (
            String::from("Glucides"),
            format!(
                "{:.0} %",
                (product.glucides_100g.unwrap_or_default() / total) * 100.0
            ),
        ),
        (
            String::from("Lipides"),
            format!(
                "{:.0} %",
                (product.lipides_100g.unwrap_or_default() / total) * 100.0
            ),
        ),
        (
            String::from("Protéines"),
            format!(
                "{:.0} %",
                (product.proteines_100g.unwrap_or_default() / total) * 100.0
            ),
        ),
    ])
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn product_image_markup(product: &Product) -> String {
    product
        .image_url
        .as_deref()
        .filter(|url| url.starts_with("https://") || url.starts_with("http://"))
        .map(|url| {
            format!(
                "<img src=\"{}\" alt=\"{}\">",
                escape_html(url),
                escape_html(&product.name)
            )
        })
        .unwrap_or_else(|| {
            "<div class=\"producer-card\">Visuel produit non renseigné</div>".to_string()
        })
}

// Générer une page HTML avec les informations de traçabilité
fn generate_traceability_html(info: &TraceabilityInfo) -> String {
    if let (Some(product), Some(batch)) = (&info.product, &info.batch) {
        let nutrition_rows = [
            ("Énergie", product.energie_kcal_100g.map(|v| format!("{v:.0} kcal"))),
            ("Glucides", product.glucides_100g.map(|v| format!("{v:.1} g"))),
            ("Lipides", product.lipides_100g.map(|v| format!("{v:.1} g"))),
            ("Protéines", product.proteines_100g.map(|v| format!("{v:.1} g"))),
        ]
        .into_iter()
        .filter_map(|(label, value)| {
            value.map(|value| {
                format!(
                    "<div class=\"info-item\"><span class=\"label\">{label} :</span><span class=\"value\">{value}</span></div>"
                )
            })
        })
        .collect::<Vec<_>>()
        .join("");
        let macro_rows = compute_macro_distribution(product)
            .unwrap_or_default()
            .into_iter()
            .map(|(label, value)| {
                format!(
                    "<div class=\"info-item\"><span class=\"label\">{label} :</span><span class=\"value\">{value}</span></div>"
                )
            })
            .collect::<Vec<_>>()
            .join("");
        let ingredient_rows = info
            .product_ingredients
            .iter()
            .map(|line| {
                format!(
                    "<div class=\"producer-card\"><strong>{}</strong><div>{}</div><div>Producteur: {} ({})</div><div>Contact: {}{}</div></div>",
                    escape_html(&line.ingredient_name),
                    escape_html(line.ingredient_category.as_deref().unwrap_or("catégorie non renseignée")),
                    escape_html(&line.producer_name),
                    escape_html(&line.producer_city),
                    escape_html(&line.producer_email),
                    line.producer_phone
                        .as_ref()
                        .map(|phone| format!(" · {}", escape_html(phone)))
                        .unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("");
        let allergen_badges = if product.allergenes.is_empty() {
            "<span class=\"value\">Aucun allergène déclaré</span>".to_string()
        } else {
            product
                .allergenes
                .iter()
                .map(|allergen| {
                    format!(
                        "<span class=\"label-badge allergen\">{}</span>",
                        escape_html(allergen)
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        };
        format!(
            r#"
<!DOCTYPE html>
<html lang="fr">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Traçabilité - {}</title>
    <style>
        body {{ font-family: Georgia, serif; margin: 0; background: linear-gradient(180deg, #f7f1e8 0%, #f3f6f0 100%); color: #2e2a24; }}
        .container {{ max-width: 860px; margin: 24px auto; background: rgba(255,255,255,0.92); padding: 32px; border-radius: 18px; box-shadow: 0 20px 45px rgba(62, 48, 32, 0.12); }}
        .header {{ text-align: center; border-bottom: 2px solid #b7732d; padding-bottom: 24px; margin-bottom: 30px; }}
        .brand {{ font-size: 14px; letter-spacing: 0.24em; text-transform: uppercase; color: #9c5c1a; margin-bottom: 8px; }}
        .product-name {{ color: #5f3a14; font-size: 32px; font-weight: bold; margin-bottom: 10px; }}
        .section {{ margin-bottom: 25px; }}
        .section h3 {{ color: #495057; border-left: 4px solid #b7732d; padding-left: 15px; margin-bottom: 15px; }}
        .info-grid {{ display: grid; gap: 10px; }}
        .info-item {{ display: flex; justify-content: space-between; padding: 8px 0; border-bottom: 1px solid #eee; }}
        .label {{ font-weight: bold; color: #495057; }}
        .value {{ color: #6c757d; }}
        .footer {{ text-align: center; margin-top: 30px; padding-top: 20px; border-top: 1px solid #eee; color: #6c757d; font-size: 14px; }}
        .labels {{ display: flex; gap: 10px; flex-wrap: wrap; }}
        .label-badge {{ background: #5f7f4a; color: white; padding: 4px 12px; border-radius: 20px; font-size: 12px; }}
        .label-badge.allergen {{ background: #a03a2d; }}
        .hero {{ display: grid; grid-template-columns: 220px 1fr; gap: 24px; align-items: center; margin-bottom: 28px; }}
        .hero img {{ width: 100%; border-radius: 16px; object-fit: cover; box-shadow: 0 10px 24px rgba(0,0,0,0.12); }}
        .nutriscore {{ display: inline-flex; gap: 8px; align-items: center; background: #fff5e8; color: #8a5200; border: 1px solid #efcfad; padding: 8px 12px; border-radius: 999px; font-weight: bold; }}
        .producer-list {{ display: grid; gap: 12px; }}
        .producer-card {{ padding: 12px 14px; border: 1px solid #e8dfd3; border-radius: 10px; background: #fff; }}
        @media (max-width: 720px) {{ .hero {{ grid-template-columns: 1fr; }} }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <div class="brand">Apothicaire Culinaire</div>
            <div class="product-name">{}</div>
            <p>Traçabilité artisanale authentifiée</p>
        </div>

        <div class="hero">
            <div>{}</div>
            <div>
                {}
                <div class="info-grid">
                    <div class="info-item">
                        <span class="label">Nom :</span>
                        <span class="value">{}</span>
                    </div>
                    <div class="info-item">
                        <span class="label">Catégorie :</span>
                        <span class="value">{}</span>
                    </div>
                </div>
            </div>
        </div>

        <div class="section">
            <h3>📦 Informations Produit</h3>
            <div class="info-grid">
                {}
                {}
                {}
            </div>
        </div>

        <div class="section">
            <h3>⚠️ Allergènes</h3>
            <div class="labels">{}</div>
        </div>

        <div class="section">
            <h3>🥄 Valeurs nutritionnelles pour 100 g</h3>
            <div class="info-grid">{}</div>
        </div>

        <div class="section">
            <h3>📊 Apport énergétique et répartition des macronutriments</h3>
            <div class="info-grid">{}</div>
        </div>

        <div class="section">
            <h3>🧑‍🌾 Producteurs impliqués</h3>
            <div class="producer-list">{}</div>
        </div>

        <div class="section">
            <h3>🏷️ Lot de Production</h3>
            <div class="info-grid">
                <div class="info-item">
                    <span class="label">Code lot :</span>
                    <span class="value">{}</span>
                </div>
                <div class="info-item">
                    <span class="label">Date de production :</span>
                    <span class="value">{}</span>
                </div>
                <div class="info-item">
                    <span class="label">DLUO/DDM :</span>
                    <span class="value">{}</span>
                </div>
                <div class="info-item">
                    <span class="label">Quantité produite :</span>
                    <span class="value">{} unités</span>
                </div>
                <div class="info-item">
                    <span class="label">Site de production :</span>
                    <span class="value">{}</span>
                </div>
                {}
            </div>
        </div>

        <div class="footer">
            <p>✅ Authentifié par Code Terroir</p>
            <p>Scan effectué le {}</p>
        </div>
    </div>
</body>
</html>
            "#,
            escape_html(&product.name),
            escape_html(&product.name),
            product_image_markup(product),
            product
                .nutriscore
                .as_ref()
                .map(|score| format!("<div class=\"nutriscore\">Nutriscore {}</div>", escape_html(score)))
                .unwrap_or_default(),
            product.description_marketing.as_ref().map_or("".to_string(), |d|
                format!("<div class=\"info-item\"><span class=\"label\">Description :</span><span class=\"value\">{}</span></div>", escape_html(d))
            ),
            if !product.labels_certifications.is_empty() {
                format!("<div class=\"info-item\"><span class=\"label\">Labels :</span><div class=\"labels\">{}</div></div>",
                    product.labels_certifications.iter()
                        .map(|l| format!("<span class=\"label-badge\">{}</span>", escape_html(l)))
                        .collect::<Vec<_>>().join(""))
            } else { "".to_string() },
            product.conseils_utilisation.as_ref().map_or("".to_string(), |c|
                format!("<div class=\"info-item\"><span class=\"label\">Conseils :</span><span class=\"value\">{}</span></div>", escape_html(c))
            ),
            escape_html(&product.name),
            escape_html(&product.category),
            allergen_badges,
            nutrition_rows,
            macro_rows,
            ingredient_rows,
            escape_html(&batch.lot_code),
            batch.production_date.format("%d/%m/%Y à %H:%M").to_string(),
            batch.dluo_ddm.format("%d/%m/%Y").to_string(),
            batch.quantity_produced,
            escape_html(&batch.production_site),
            batch.notes.as_ref().map_or("".to_string(), |n|
                format!("<div class=\"info-item\"><span class=\"label\">Notes :</span><span class=\"value\">{}</span></div>", escape_html(n))
            ),
            chrono::Utc::now().format("%d/%m/%Y à %H:%M UTC").to_string()
        )
    } else {
        generate_error_html("inconnu")
    }
}

// Générer une page HTML d'erreur
fn generate_error_html(slug: &str) -> String {
    format!(
        r#"
<!DOCTYPE html>
<html lang="fr">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Code non trouvé - Code Terroir</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; background: #f8f9fa; color: #333; text-align: center; }}
        .container {{ max-width: 500px; margin: 0 auto; background: white; padding: 40px; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }}
        .error-icon {{ font-size: 64px; margin-bottom: 20px; }}
        .error-title {{ color: #dc3545; font-size: 24px; font-weight: bold; margin-bottom: 15px; }}
        .error-message {{ color: #6c757d; margin-bottom: 30px; }}
        .code {{ background: #f8f9fa; padding: 10px; border-radius: 5px; font-family: monospace; font-weight: bold; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="error-icon">❌</div>
        <h1 class="error-title">Code non trouvé</h1>
        <p class="error-message">
            Le code de traçabilité que vous avez scanné n'existe pas ou n'est plus actif.
        </p>
        <div class="code">{}</div>
        <p style="margin-top: 30px; color: #6c757d; font-size: 14px;">
            Si vous pensez qu'il s'agit d'une erreur, contactez le producteur.
        </p>
    </div>
</body>
</html>
        "#,
        escape_html(slug)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_escaping_neutralizes_markup_and_attributes() {
        assert_eq!(
            escape_html("<script>alert('xss') & \"more\"</script>"),
            "&lt;script&gt;alert(&#x27;xss&#x27;) &amp; &quot;more&quot;&lt;/script&gt;"
        );
        assert!(!generate_error_html("<img src=x onerror=alert(1)>")
            .contains("<img src=x onerror=alert(1)>"));
    }

    #[test]
    fn parse_ip_network_supports_plain_addresses() {
        assert!(matches!(
            parse_ip_network(Some("203.0.113.10")),
            Some(IpNetwork::V4(_))
        ));
        assert!(matches!(
            parse_ip_network(Some("2001:db8::abcd")),
            Some(IpNetwork::V6(_))
        ));
    }

    #[test]
    fn parse_ip_network_rejects_invalid_values() {
        assert_eq!(parse_ip_network(None), None);
        assert_eq!(parse_ip_network(Some("invalid-ip")), None);
    }

    #[test]
    fn generate_error_html_includes_slug() {
        let html = generate_error_html("abc123");
        assert!(html.contains("abc123"));
        assert!(html.contains("Code non trouvé"));
    }

    #[test]
    fn compute_macro_distribution_returns_percentages() {
        let product = Product {
            id: Uuid::new_v4(),
            producer_id: Uuid::new_v4(),
            name: String::from("Test"),
            category: String::from("confiture"),
            recipe_id: None,
            description_marketing: None,
            ingredients: None,
            nutritional_values: None,
            image_url: None,
            nutriscore: Some(String::from("B")),
            energie_kcal_100g: Some(210.0),
            glucides_100g: Some(50.0),
            lipides_100g: Some(30.0),
            proteines_100g: Some(20.0),
            allergenes: vec![],
            labels_certifications: vec![],
            visuels: vec![],
            conseils_utilisation: None,
            infos_legales: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let ratios = compute_macro_distribution(&product).expect("macro ratios");
        assert_eq!(ratios[0].1, "50 %");
        assert_eq!(ratios[1].1, "30 %");
        assert_eq!(ratios[2].1, "20 %");
    }
}
