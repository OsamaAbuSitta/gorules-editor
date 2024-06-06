use std::env;
use std::path::Path;
use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{extract::Path as AxumPath, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::runtime::Handle;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_status::SetStatus;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use zen_engine::model::DecisionContent;
use zen_engine::{DecisionEngine, DecisionGraphResponse, EvaluationError, EvaluationOptions};
use postgres::{Client , NoTls};
use postgres::Error as postgresError;
use std::fs::File;
use std::io::Write;
use std::fs;


#[macro_use]
extern crate serde_json;


//Model 


const IS_DEVELOPMENT: bool = cfg!(debug_assertions);

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "editor=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let host_address = IS_DEVELOPMENT.then_some("127.0.0.1").unwrap_or("0.0.0.0");
    let listener_address = format!("{host_address}:3000");

    let app = Router::new()
        .route("/api/health", get(health))
        .route(
            "/api/simulate",
            post(simulate).layer(DefaultBodyLimit::max(16 * 1024 * 1024)),
        ).route(
            "/api/rules/files",
            get(list_files).layer(DefaultBodyLimit::max(16 * 1024 * 1024)).post(save_rule_json),
        ) .route("/api/rules/files/:name", get(get_file_by_name))
        .route("/api/evaluate", post(evaluate_file))
        .nest_service("/", serve_dir_service());

    let listener = tokio::net::TcpListener::bind(listener_address)
        .await
        .unwrap();
    let compression_layer = CompressionLayer::new().gzip(true).br(true);

    tracing::info!("🚀 Listening on http://{}", listener.local_addr().unwrap());

    let mut app_with_layers = app
        .layer(TraceLayer::new_for_http())
        .layer(compression_layer);
    if let Ok(_) = env::var("CORS_PERMISSIVE") {
        app_with_layers = app_with_layers.layer(CorsLayer::permissive())
    }

    axum::serve(listener, app_with_layers).await.unwrap();
}

fn serve_dir_service() -> ServeDir<SetStatus<ServeFile>> {
    let work_dir = env::current_dir().ok().map_or("static".to_string(), |dir| {
        dir.to_string_lossy().to_string()
    });
    let static_path = Path::new(&work_dir).join("static");
    let index_path = static_path.join("index.html");

    ServeDir::new(static_path).not_found_service(ServeFile::new(index_path))
}

async fn health() -> (StatusCode, String) {
    (StatusCode::OK, String::from("healthy"))
}

#[derive(Deserialize, Serialize)]
struct SimulateRequest {
    context: Value,
    content: DecisionContent,
}

#[derive(Deserialize, Serialize)]
struct FileEvaluationRequest {
    context: Value,
    file_name: String,
}


async fn simulate(
    Json(payload): Json<SimulateRequest>,
) -> Result<Json<DecisionGraphResponse>, SimulateError> {
    let engine = DecisionEngine::default();
    let decision = engine.create_decision(payload.content.into());
    let result = tokio::task::spawn_blocking(move || {
        Handle::current().block_on(decision.evaluate_with_opts(
            &payload.context,
            EvaluationOptions {
                trace: Some(true),
                max_depth: None,
            },
        ))
    })
    .await
    .unwrap()?;

    return Ok(Json(result));
}


struct SimulateError(Box<EvaluationError>);

impl IntoResponse for SimulateError {
    fn into_response(self) -> Response {
        (
            StatusCode::BAD_REQUEST,
            serde_json::to_string(&self.0).unwrap_or_default(),
        )
            .into_response()
    }
}

impl From<Box<EvaluationError>> for SimulateError {
    fn from(value: Box<EvaluationError>) -> Self {
        Self(value)
    }
}


/* =============================================================================================== */
#[derive(Deserialize, Serialize)]
struct SaveRuleRequest {
    name: String , 
    json: String , 
}

async fn save_rule_json(Json(payload): Json<SaveRuleRequest>)-> impl IntoResponse 
{
    let SaveRuleRequest {name, json } = payload;

    let file_path = Path::new("saved_files").join(name);

    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }

    match File::create(&file_path) {
        Ok(mut file) => match file.write_all(json.as_bytes()) {
            Ok(_) => (StatusCode::OK, "File saved successfully").into_response(),
            Err(err) => {
                eprintln!("Failed to write to file: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save file").into_response()
            }
        },
        Err(err) => {
            eprintln!("Failed to create file: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save file").into_response()
        }
    }
}

#[derive(Deserialize, Serialize)]
struct FileContentResponse {
    name: String,
    content: String,
}

async fn get_file_by_name(AxumPath(name): AxumPath<String>) -> impl IntoResponse {
    let file_path = Path::new("saved_files").join(&name);

    match fs::read_to_string(&file_path) {
        Ok(content) => {
            let response = FileContentResponse {
                name: name.clone(),
                content,
            };
            Json(response).into_response()
        }
        Err(err) => {
            eprintln!("Failed to read file {}: {}", name, err);
            (StatusCode::NOT_FOUND, "File not found").into_response()
        }
    }
}

async fn list_files() -> impl IntoResponse {
    let files_dir = Path::new("saved_files");

    // Create the directory if it doesn't exist
    if !files_dir.exists() {
        std::fs::create_dir_all(files_dir).unwrap();
    }

    match fs::read_dir(files_dir) {
        Ok(entries) => {
            let file_names: Vec<String> = entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| entry.path().file_name().map(|name| name.to_string_lossy().into_owned()))
                .collect();

            Json(file_names).into_response()
        }
        Err(err) => {
            eprintln!("Failed to read directory: {}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to list files").into_response()
        }
    }
}


/* =============================================================================================== */
async fn evaluate_file(Json(payload): Json<FileEvaluationRequest>) -> impl IntoResponse {
    let file_path = Path::new("saved_files").join(&payload.file_name);

    match fs::read_to_string(&file_path) {
        Ok(file_content) => {
            let decision_content: DecisionContent = serde_json::from_str(&file_content).expect("Can't read rule file");

            let engine = DecisionEngine::default();
            let decision = engine.create_decision(decision_content.into());
            let result = tokio::task::spawn_blocking(move || {
                Handle::current().block_on(decision.evaluate_with_opts(
                    &payload.context,
                    EvaluationOptions {
                        trace: Some(false),
                        max_depth: None,
                    },
                ))
            })
            .await
            .unwrap();

            Json(result).into_response()
            //Ok(Json(result))
        }
        Err(err) => {
            eprintln!("Failed to read file {}: {}", payload.file_name, err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save file").into_response()
        }
    }
}


