//! HTTP server common module
//!
//! Shared types, constants, and handlers used by both share and web_upload servers.

use axum::{
    body::Body,
    extract::{connect_info::ConnectInfo, State as AxumState},
    http::{header, HeaderName, StatusCode},
    response::{Html, IntoResponse, Json, Response},
};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

use crate::transfer::compression::get_compression_config;
use crate::transfer::crypto::is_encryption_enabled;
use crate::transfer::http_crypto::{
    HandshakeRequest, HandshakeResponse, HttpCryptoSessionManager,
};

// ─── Shared Constants ───────────────────────────────────────────────────────

pub static FAVICON_ICO: &[u8] = include_bytes!("../../icons/32x32.png");

pub const HTTP_CHUNK_SIZE: usize = 1024 * 1024; // 1MB

pub const SESSION_CLEANUP_INTERVAL_SECS: u64 = 300;

// ─── Shared Types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ServerCapabilities {
    pub encryption: bool,
    pub compression: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compression_algorithm: Option<String>,
    pub chunk_size: usize,
}

impl ServerCapabilities {
    pub fn for_share() -> Self {
        let encryption = is_encryption_enabled();
        let compression_config = get_compression_config();
        Self {
            encryption,
            compression: compression_config.enabled,
            compression_algorithm: if compression_config.enabled {
                Some("zstd".to_string())
            } else {
                None
            },
            chunk_size: HTTP_CHUNK_SIZE,
        }
    }

    pub fn for_web_upload() -> Self {
        let encryption = is_encryption_enabled();
        let compression_config = get_compression_config();
        Self {
            encryption,
            compression: compression_config.enabled,
            compression_algorithm: None,
            chunk_size: HTTP_CHUNK_SIZE,
        }
    }
}

// ─── Trait for crypto session access ────────────────────────────────────────

pub trait HasCryptoSessions {
    fn crypto_sessions(&self) -> &Arc<Mutex<HttpCryptoSessionManager>>;
}

// ─── Shared Handlers ────────────────────────────────────────────────────────

pub async fn favicon_handler() -> impl IntoResponse {
    let mut response = Response::new(Body::from(FAVICON_ICO));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("image/png"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("max-age=86400"),
    );
    response
}

pub async fn crypto_handshake_handler<S: HasCryptoSessions + Send + Sync + 'static>(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<Arc<S>>,
    Json(payload): Json<HandshakeRequest>,
) -> Json<HandshakeResponse> {
    if !is_encryption_enabled() {
        return Json(HandshakeResponse {
            encryption: false,
            server_public_key: None,
            session_id: None,
        });
    }

    let client_ip = client_addr.ip().to_string();
    let mut crypto_sessions = state.crypto_sessions().lock().await;

    match crypto_sessions.handshake(&payload.client_public_key, client_ip) {
        Ok((session_id, server_pub_key)) => Json(HandshakeResponse {
            encryption: true,
            server_public_key: Some(server_pub_key),
            session_id: Some(session_id),
        }),
        Err(e) => {
            eprintln!("Crypto handshake failed: {}", e);
            Json(HandshakeResponse {
                encryption: false,
                server_public_key: None,
                session_id: None,
            })
        }
    }
}

pub async fn fallback_handler(uri: axum::http::Uri) -> impl IntoResponse {
    eprintln!("Unmatched route: {}", uri);
    (
        StatusCode::NOT_FOUND,
        Html(format!(
            "<html><body><h1>404 - Not Found</h1><p>Path: {}</p></body></html>",
            uri
        )),
    )
}

// ─── Session Cleanup ────────────────────────────────────────────────────────

pub fn spawn_crypto_session_cleanup(crypto_sessions: Arc<Mutex<HttpCryptoSessionManager>>) {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(SESSION_CLEANUP_INTERVAL_SECS));
        loop {
            interval.tick().await;
            crypto_sessions.lock().await.cleanup_expired();
        }
    });
}

// ─── CORS Configuration ─────────────────────────────────────────────────────

/// Create a CORS layer with the given allowed and exposed headers.
///
/// Both share and web_upload servers use the same CORS pattern (allow any origin,
/// GET + POST methods) but differ in which custom headers they need.
pub fn create_cors_layer(
    allow_headers: Vec<HeaderName>,
    expose_headers: Vec<HeaderName>,
) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers(allow_headers)
        .expose_headers(expose_headers)
}

/// CORS layer for the share (download) server.
pub fn share_cors_layer() -> CorsLayer {
    create_cors_layer(
        vec![
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::RANGE,
            HeaderName::from_static("x-encryption-session"),
        ],
        vec![
            header::CONTENT_RANGE,
            header::ACCEPT_RANGES,
            header::ETAG,
            HeaderName::from_static("x-chunk-index"),
            HeaderName::from_static("x-original-size"),
            HeaderName::from_static("x-compression"),
            HeaderName::from_static("x-encryption"),
        ],
    )
}

/// CORS layer for the web upload server.
pub fn web_upload_cors_layer() -> CorsLayer {
    create_cors_layer(
        vec![
            header::CONTENT_TYPE,
            header::ACCEPT,
            HeaderName::from_static("x-upload-id"),
            HeaderName::from_static("x-chunk-index"),
            HeaderName::from_static("x-encryption-session"),
            HeaderName::from_static("x-compression"),
        ],
        vec![HeaderName::from_static("x-file-hash")],
    )
}

// ─── HTML Utilities ─────────────────────────────────────────────────────────

pub fn parse_user_agent(ua: &str) -> &'static str {
    let ua_lower = ua.to_lowercase();

    let platform = if ua_lower.contains("android") {
        "Android"
    } else if ua_lower.contains("iphone") || ua_lower.contains("ipad") || ua_lower.contains("ipod")
    {
        "iOS"
    } else if ua_lower.contains("mac") || ua_lower.contains("macos") {
        "macOS"
    } else if ua_lower.contains("windows") {
        "Windows"
    } else if ua_lower.contains("linux") {
        "Linux"
    } else {
        "Unknown"
    };

    let browser = if ua_lower.contains("edg/") || ua_lower.contains("edge") {
        "Edge"
    } else if ua_lower.contains("opr/") || ua_lower.contains("opera") {
        "Opera"
    } else if ua_lower.contains("firefox") || ua_lower.contains("fxios") {
        "Firefox"
    } else if ua_lower.contains("chrome") || ua_lower.contains("crios") {
        "Chrome"
    } else if ua_lower.contains("safari") && !ua_lower.contains("chrome") {
        "Safari"
    } else if ua_lower.contains("msie") || ua_lower.contains("trident") {
        "IE"
    } else {
        "Unknown"
    };

    match (browser, platform) {
        ("Chrome", "Android") => "Chrome(Android)",
        ("Chrome", "iOS") => "Chrome(iOS)",
        ("Chrome", "macOS") => "Chrome(macOS)",
        ("Chrome", "Windows") => "Chrome(Windows)",
        ("Chrome", "Linux") => "Chrome(Linux)",
        ("Chrome", _) => "Chrome",
        ("Safari", "iOS") => "Safari(iOS)",
        ("Safari", "macOS") => "Safari(macOS)",
        ("Safari", _) => "Safari",
        ("Firefox", "Android") => "Firefox(Android)",
        ("Firefox", "iOS") => "Firefox(iOS)",
        ("Firefox", _) => "Firefox",
        ("Edge", "Windows") => "Edge(Windows)",
        ("Edge", "macOS") => "Edge(macOS)",
        ("Edge", _) => "Edge",
        ("Opera", "Android") => "Opera(Android)",
        ("Opera", _) => "Opera",
        ("IE", _) => "IE",
        (_, "Android") => "Browser(Android)",
        (_, "iOS") => "Browser(iOS)",
        (_, "macOS") => "Browser(macOS)",
        (_, "Windows") => "Browser(Windows)",
        (_, "Linux") => "Browser(Linux)",
        (_, _) => "Browser",
    }
}
