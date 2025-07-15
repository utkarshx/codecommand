use std::{str::FromStr, sync::Arc};

use axum::{
    body::Body,
    http::{header, HeaderValue, StatusCode},
    middleware::from_fn_with_state,
    response::{IntoResponse, Json as ResponseJson, Response},
    routing::{get, post},
    Json, Router,
};
use sentry_tower::NewSentryLayer;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use strip_ansi_escapes::strip;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{filter::LevelFilter, prelude::*};
use codecommand::{sentry_layer, Assets, ScriptAssets, SoundAssets};

mod app_state;
mod execution_monitor;
mod executor;
mod executors;
mod mcp;
mod models;
mod routes;
mod services;
mod utils;

use app_state::AppState;
use execution_monitor::execution_monitor;
use models::{ApiResponse, Config};
use routes::{auth, config, filesystem, health, projects, task_attempts, tasks};
use services::PrMonitorService;

async fn echo_handler(
    Json(payload): Json<serde_json::Value>,
) -> ResponseJson<ApiResponse<serde_json::Value>> {
    ResponseJson(ApiResponse {
        success: true,
        data: Some(payload),
        message: Some("Echo successful".to_string()),
    })
}

async fn static_handler(uri: axum::extract::Path<String>) -> impl IntoResponse {
    let path = uri.trim_start_matches('/');
    serve_file(path).await
}

async fn index_handler() -> impl IntoResponse {
    serve_file("index.html").await
}

async fn serve_file(path: &str) -> impl IntoResponse {
    let file = Assets::get(path);

    match file {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();

            Response::builder()
                .status(StatusCode::OK)
                .header(
                    header::CONTENT_TYPE,
                    HeaderValue::from_str(mime.as_ref()).unwrap(),
                )
                .body(Body::from(content.data.into_owned()))
                .unwrap()
        }
        None => {
            // For SPA routing, serve index.html for unknown routes
            if let Some(index) = Assets::get("index.html") {
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, HeaderValue::from_static("text/html"))
                    .body(Body::from(index.data.into_owned()))
                    .unwrap()
            } else {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("404 Not Found"))
                    .unwrap()
            }
        }
    }
}

async fn serve_sound_file(
    axum::extract::Path(filename): axum::extract::Path<String>,
) -> impl IntoResponse {
    // Validate filename contains only expected sound files
    let valid_sounds = [
        "abstract-sound1.wav",
        "abstract-sound2.wav",
        "abstract-sound3.wav",
        "abstract-sound4.wav",
        "cow-mooing.wav",
        "phone-vibration.wav",
        "rooster.wav",
    ];

    if !valid_sounds.contains(&filename.as_str()) {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Sound file not found"))
            .unwrap();
    }

    match SoundAssets::get(&filename) {
        Some(content) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, HeaderValue::from_static("audio/wav"))
            .body(Body::from(content.data.into_owned()))
            .unwrap(),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Sound file not found"))
            .unwrap(),
    }
}

fn main() -> anyhow::Result<()> {
    let _environment = if cfg!(debug_assertions) {
        "dev"
    } else {
        "production"
    };
    
    // Force disable Sentry - use dummy configuration
    let _guard = sentry::init(sentry::ClientOptions {
        dsn: None, // Disable Sentry by setting DSN to None
        ..Default::default()
    });
    
    /*
    let _guard = sentry::init(("https://1065a1d276a581316999a07d5dffee26@o4509603705192449.ingest.de.sentry.io/4509605576441937", sentry::ClientOptions {
        release: sentry::release_name!(),
        environment: Some(environment.into()),
        attach_stacktrace: true,
        ..Default::default()
    }));
    */
    
    sentry::configure_scope(|scope| {
        scope.set_tag("source", "server");
    });
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer().with_filter(LevelFilter::INFO))
                .with(sentry_layer())
                .init();

            // Create asset directory if it doesn't exist
            if !utils::asset_dir().exists() {
                std::fs::create_dir_all(utils::asset_dir())?;
            }

            // Database connection
            let database_url = format!(
                "sqlite://{}",
                utils::asset_dir().join("db.sqlite").to_string_lossy()
            );

            let options = SqliteConnectOptions::from_str(&database_url)?.create_if_missing(true);
            let pool = SqlitePool::connect_with(options).await?;
            sqlx::migrate!("./migrations").run(&pool).await?;

            // Load configuration
            let config_path = utils::config_path();
            let config = Config::load(&config_path)?;
            let config_arc = Arc::new(RwLock::new(config));

            // Create app state
            let app_state = AppState::new(pool.clone(), config_arc.clone()).await;

            app_state.update_sentry_scope().await;

            // Track session start event
            app_state.track_analytics_event("session_start", None).await;
            // Start background task to check for init status and spawn processes
            let state_clone = app_state.clone();
            tokio::spawn(async move {
                execution_monitor(state_clone).await;
            });

            // Start PR monitoring service
            let pr_monitor = PrMonitorService::new(pool.clone());
            let config_for_monitor = config_arc.clone();

            tokio::spawn(async move {
                pr_monitor.start_with_config(config_for_monitor).await;
            });

            // Public routes (no auth required)
            let public_routes = Router::new()
                .route("/api/health", get(health::health_check))
                .route("/api/echo", post(echo_handler));

            // All routes (no auth required)
            let app_routes = Router::new()
                .nest(
                    "/api",
                    Router::new()
                        .merge(projects::projects_router())
                        .merge(tasks::tasks_router())
                        .merge(task_attempts::task_attempts_router())
                        .merge(filesystem::filesystem_router())
                        .merge(config::config_router())
                        .merge(auth::auth_router())
                        .route("/sounds/:filename", get(serve_sound_file))
                        .layer(from_fn_with_state(app_state.clone(), auth::sentry_user_context_middleware)),
                );

            let app = Router::new()
                .merge(public_routes)
                .merge(app_routes)
                // Static file serving routes
                .route("/", get(index_handler))
                .route("/*path", get(static_handler))
                .with_state(app_state)
                .layer(CorsLayer::permissive())
                .layer(NewSentryLayer::new_from_top());

            let port = std::env::var("BACKEND_PORT")
                .or_else(|_| std::env::var("PORT"))
                .ok()
                .and_then(|s| {
                    // remove any ANSI codes, then turn into String
                    let cleaned = String::from_utf8(strip(s.as_bytes()))
                        .expect("UTF-8 after stripping ANSI");
                    cleaned.trim().parse::<u16>().ok()
                })
                .unwrap_or_else(|| {
                    tracing::error!("Failed to parse port after stripping ANSI, defaulting to 0");
                    0
                }); // Use 0 to find free port if no specific port provided

            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
            let actual_port = listener.local_addr()?.port(); // get → 53427 (example)

            tracing::info!("Server running on http://0.0.0.0:{actual_port}");

            if !cfg!(debug_assertions) {
                tracing::info!("Opening browser...");
                if let Err(e) = utils::open_browser(&format!("http://127.0.0.1:{actual_port}")).await {
                    tracing::warn!("Failed to open browser automatically: {}. Please open http://127.0.0.1:{} manually.", e, actual_port);
                }
            }

            axum::serve(listener, app).await?;

            Ok(())
        })
}
