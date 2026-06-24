use axum::{Extension, Router};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa_swagger_ui::SwaggerUi;

use crate::app_state::AppState;
use crate::controllers::api_routes;
use crate::docs::ApiDoc;
use crate::middleware::{payload_encryption, verify_api_gateway_key};
use utoipa::OpenApi;

/// Build the Axum Router configuring routes, CORS, logging, and Swagger UI.
pub fn create_router(state: AppState) -> Router {
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    let cors = if app_env == "production" {
        let mut allowed_origins = Vec::new();
        for domain in state.config.domain_name.split(',') {
            let trimmed = domain.trim();
            if !trimmed.is_empty()
                && let Ok(origin) = trimmed.parse::<axum::http::HeaderValue>()
            {
                allowed_origins.push(origin);
            }
        }
        CorsLayer::new()
            .allow_origin(allowed_origins)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
    } else {
        CorsLayer::permissive()
    };

    let api_routes = Router::new()
        // Mount all API routes under /api
        .nest("/api", api_routes())
        // Apply Gateway Key validation middleware (Runs inner to encryption)
        .layer(axum::middleware::from_fn(verify_api_gateway_key))
        .layer(axum::middleware::from_fn(payload_encryption));

    let mut router = Router::new();
    // Serve OpenAPI document & Swagger UI automatically at /api-docs ONLY in non-production environments
    if app_env != "production" {
        router = router
            .merge(SwaggerUi::new("/api-docs").url("/api-docs/openapi.json", ApiDoc::openapi()));
    }

    router
        .merge(api_routes)
        // Add tracing/logging layer
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        // Injected via Extension so the JWT extractor can access AppConfig & AppState
        .layer(Extension(state.config.clone()))
        .layer(Extension(state.clone()))
        .with_state(state)
}
