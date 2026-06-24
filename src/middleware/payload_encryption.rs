use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use serde_json::Value;

use crate::services::EncryptionService;

pub async fn payload_encryption(mut req: Request, next: Next) -> Result<Response, StatusCode> {
    if !EncryptionService::encryption_enabled() {
        return Ok(next.run(req).await);
    }

    // 1. Decrypt Request Body
    let method = req.method().clone();
    if method == axum::http::Method::POST
        || method == axum::http::Method::PUT
        || method == axum::http::Method::PATCH
        || method == axum::http::Method::DELETE
    {
        // Extract the body
        let body = std::mem::replace(req.body_mut(), Body::empty());
        let bytes = axum::body::to_bytes(body, 2 * 1024 * 1024)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        if !bytes.is_empty() {
            if let Ok(json_body) = serde_json::from_slice::<Value>(&bytes) {
                let payload_str = if let Some(payload) = json_body.get("payload") {
                    payload.as_str()
                } else {
                    json_body.as_str()
                };

                if let Some(encoded) = payload_str {
                    match EncryptionService::decrypt(encoded) {
                        Ok(decrypted) => {
                            // Inject decrypted body back into request
                            *req.body_mut() = Body::from(decrypted);
                            req.headers_mut().insert(
                                header::CONTENT_TYPE,
                                HeaderValue::from_static("application/json"),
                            );
                        }
                        Err(e) => {
                            tracing::warn!("Failed to decrypt payload: {}", e);
                            return Err(StatusCode::BAD_REQUEST);
                        }
                    }
                } else {
                    // Body exists but doesn't have "payload" field or isn't a string.
                    // Put it back as is (useful for debugging like Rails implementation)
                    *req.body_mut() = Body::from(bytes);
                }
            } else {
                *req.body_mut() = Body::from(bytes);
            }
        }
    }

    // 2. Process Request
    let response = next.run(req).await;

    // 3. Encrypt Response Body
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if content_type.contains("application/json") {
        let (mut parts, body) = response.into_parts();

        let bytes = match axum::body::to_bytes(body, 10 * 1024 * 1024).await {
            Ok(b) => b,
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        };

        if !bytes.is_empty()
            && let Ok(response_str) = String::from_utf8(bytes.to_vec())
        {
            match EncryptionService::encrypt(&response_str) {
                Ok(encrypted) => {
                    let new_body = serde_json::json!({ "data": encrypted }).to_string();
                    parts.headers.insert(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("application/json; charset=utf-8"),
                    );
                    // Content length automatically set by axum for Body::from(String)
                    return Ok((parts, Body::from(new_body)).into_response());
                }
                Err(e) => {
                    tracing::error!("Failed to encrypt response: {}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }

        Ok((parts, Body::from(bytes)).into_response())
    } else {
        Ok(response)
    }
}
