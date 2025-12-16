use axum::{
    Router,
    extract::{Json, State},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use axum_csrf::{CsrfConfig, CsrfLayer, CsrfToken};
use clap::Parser;
use reqwest::StatusCode;
use serde::Deserialize;
use std::net::SocketAddr;

mod db;
mod state;
mod static_files;
use static_files::{CSS, INDEX_HTML, JS};

use crate::{
    db::InvitesRepo,
    state::{AppState, Args},
};

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    password: String,
    invite_code: String,
    csrf_token: String,
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
    username: String,
    password: String,
    new_password: String,
    csrf_token: String,
}

#[derive(Deserialize)]
struct AdminRequest {
    admin_token: String,
}

#[tokio::main]
async fn main() {
    let csrf_layer = CsrfLayer::new(CsrfConfig::default());

    dotenvy::dotenv().expect("Failed to load env vars");
    let args = Args::parse();

    let state = AppState {
        matrix_server_url: args.matrix_server_url,
        matrix_reg_token: args.matrix_reg_token,
        repo: InvitesRepo::new(),
        admin_token: args.admin_token,
    };

    let app = Router::new()
        .route("/", get(root).post(csrf))
        .route("/bundle.js", get(js))
        .route("/bundle.css", get(css))
        .route("/api/register", post(register))
        .route("/api/change_password", post(change_password))
        .route("/api/invite/new", post(generate_invite))
        .route("/api/invite/", post(active_invites))
        .layer(csrf_layer)
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

async fn root(token: CsrfToken) -> impl IntoResponse {
    (token, Html(INDEX_HTML))
}

async fn csrf(token: CsrfToken) -> impl IntoResponse {
    let token = token.authenticity_token().unwrap();
    token
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
async fn register(
    State(state): State<AppState>,
    token: CsrfToken,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    if token.verify(&payload.csrf_token).is_err() {
        return (StatusCode::FORBIDDEN, "CSRF invalid".to_owned());
    }

    if let Some(invite_is_active) = state.repo.check_invite(&payload.invite_code) {
        if invite_is_active {
            return (
                StatusCode::FORBIDDEN,
                "Invite is wrong or already used".to_string(),
            );
        }
    } else {
        return (
            StatusCode::FORBIDDEN,
            "Invite is wrong or already used".to_string(),
        );
    }

    let token = state.matrix_reg_token;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "{}/_matrix/client/r0/register",
            state.matrix_server_url
        ))
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

    let response = match resp {
        Ok(r) if r.status().is_success() => "User registered successfully".to_string(),
        Ok(r) => format!("Error: {}", r.text().await.unwrap_or_default()),
        Err(e) => format!("Request failed: {}", e),
    };

    state.repo.use_invite(&payload.invite_code);
    (StatusCode::OK, response)
}

#[axum::debug_handler]
async fn change_password(
    State(state): State<AppState>,
    token: CsrfToken,
    Json(payload): Json<ChangePasswordRequest>,
) -> impl IntoResponse {
    if token.verify(&payload.csrf_token).is_err() {
        return (StatusCode::FORBIDDEN, "CSRF invalid".to_owned());
    }

    let client = reqwest::Client::new();

    // 1. Логинимся
    let login_resp = client
        .post(format!(
            "{}/_matrix/client/r0/login",
            state.matrix_server_url
        ))
        .json(&serde_json::json!({
            "type": "m.login.password",
            "user": payload.username,
            "password": payload.password
        }))
        .send()
        .await;

    let login_json = match login_resp {
        Ok(r) if r.status().is_success() => r.json::<serde_json::Value>().await.unwrap(),
        Ok(r) => {
            return (
                StatusCode::FORBIDDEN,
                format!("Login failed: {}", r.text().await.unwrap_or_default()),
            );
        }
        Err(e) => {
            return (
                StatusCode::FORBIDDEN,
                format!("Login request failed: {}", e),
            );
        }
    };

    let access_token = login_json["access_token"].as_str().unwrap();

    // 2. Смена пароля
    let resp = client
        .post(format!(
            "{}/_matrix/client/v3/account/password",
            state.matrix_server_url
        ))
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
        Ok(r) if r.status().is_success() => (
            StatusCode::ACCEPTED,
            "Password changed successfully".to_string(),
        ),
        Ok(r) => (
            StatusCode::FORBIDDEN,
            format!("Error: {}", r.text().await.unwrap_or_default()),
        ),
        Err(e) => (StatusCode::FORBIDDEN, format!("Request failed: {}", e)),
    }
}

async fn generate_invite(
    State(state): State<AppState>,
    Json(payload): Json<AdminRequest>,
) -> impl IntoResponse {
    if payload.admin_token != state.admin_token {
        return (StatusCode::FORBIDDEN, "ERRROR".to_owned());
    }
    let invite_code = state.repo.new_invite();

    (StatusCode::ACCEPTED, invite_code)
}

async fn active_invites(
    State(state): State<AppState>,
    Json(payload): Json<AdminRequest>,
) -> impl IntoResponse {
    if payload.admin_token != state.admin_token {
        return (StatusCode::FORBIDDEN, "ERRROR".to_owned());
    }

    let results = state.repo.active_invites();
    (StatusCode::ACCEPTED, results)
}
