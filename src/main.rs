use axum::{
    Router,
    extract::{Json, State},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use clap::Parser;
use serde::Deserialize;
use std::net::SocketAddr;

mod state;
mod static_files;
use static_files::{CSS, INDEX_HTML, JS};

use crate::state::{AppState, Args};

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
    username: String,
    password: String,
    new_password: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Failed to load env vars");
    let args = Args::parse();

    let state = AppState {
        matrix_domain: args.matrix_domain,
        matrix_reg_token: args.matrix_reg_token,
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/bundle.js", get(js))
        .route("/bundle.css", get(css))
        .route("/api/register", post(register))
        .route("/api/change_password", post(change_password))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to create listener");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn root() -> impl IntoResponse {
    Html(INDEX_HTML)
}

async fn js() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "application/javascript")],
        JS,
    )
}

async fn css() -> impl IntoResponse {
    ([(axum::http::header::CONTENT_TYPE, "text/css")], CSS)
}

#[axum::debug_handler]
async fn register(State(state): State<AppState>, Json(payload): Json<RegisterRequest>) -> impl IntoResponse {
    let token = state.matrix_reg_token;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("https://{}/_matrix/client/r0/register", state.matrix_domain))
        .json(&serde_json::json!({
            "username": payload.username,
            "password": payload.password,
            "auth": {
                "type": "m.login.registration_token",
                "token": token,
            }
        }))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => "User registered successfully".to_string(),
        Ok(r) => format!("Error: {}", r.text().await.unwrap_or_default()),
        Err(e) => format!("Request failed: {}", e),
    }
}

#[axum::debug_handler]
async fn change_password(State(state): State<AppState>, Json(payload): Json<ChangePasswordRequest>) -> impl IntoResponse {
    let client = reqwest::Client::new();

    // 1. Логинимся
    let login_resp = client
        .post(format!("https://{}/_matrix/client/r0/login", state.matrix_domain))
        .json(&serde_json::json!({
            "type": "m.login.password",
            "user": payload.username,
            "password": payload.password
        }))
        .send()
        .await;

    let login_json = match login_resp {
        Ok(r) if r.status().is_success() => r.json::<serde_json::Value>().await.unwrap(),
        Ok(r) => return format!("Login failed: {}", r.text().await.unwrap_or_default()),
        Err(e) => return format!("Login request failed: {}", e),
    };

    let access_token = login_json["access_token"].as_str().unwrap();

    // 2. Смена пароля
    let resp = client
        .post(format!("https://{}/_matrix/client/v3/account/password", state.matrix_domain))
        .bearer_auth(access_token)
        .json(&serde_json::json!({
            "auth": {
                "type": "m.login.password",
                "user": payload.username,
                "password": payload.password
            },
            "new_password": payload.new_password
        }))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => "Password changed successfully".to_string(),
        Ok(r) => format!("Error: {}", r.text().await.unwrap_or_default()),
        Err(e) => format!("Request failed: {}", e),
    }
}
