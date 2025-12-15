use axum::{
    Router,
    extract::{Json, State},
    response::{Html, IntoResponse},
    routing::{get, post},
};
use axum_csrf::{CsrfConfig, CsrfLayer, CsrfToken};
use clap::Parser;
use rand::{Rng, distr::Alphanumeric, rng};
use reqwest::StatusCode;
use rocksdb::{ColumnFamilyDescriptor, DB, IteratorMode, Options};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};

mod state;
mod static_files;
use static_files::{CSS, INDEX_HTML, JS};

use crate::state::{AppState, Args};

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

fn open_db() -> DB {
    let mut opts = Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);

    let cf_invites = ColumnFamilyDescriptor::new("invites", Options::default());

    let db = DB::open_cf_descriptors(&opts, "portaldb", vec![cf_invites]).unwrap();

    return db;
}

#[tokio::main]
async fn main() {
    let csrf_layer = CsrfLayer::new(CsrfConfig::default());

    let db = open_db();

    dotenvy::dotenv().expect("Failed to load env vars");
    let args = Args::parse();

    let state = AppState {
        matrix_server_url: args.matrix_server_url,
        matrix_reg_token: args.matrix_reg_token,
        db: Arc::new(db),
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
    let db = state.db.clone();
    let cf = db.cf_handle("invites").unwrap();

    if let Ok(Some(invite_is_active)) = db.get_cf(&cf, format!("INVITE:{}", payload.invite_code)) {
        if invite_is_active == &[1] {
            return (StatusCode::FORBIDDEN, "Invite is already used".to_string());
        }
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


    db.put_cf(&cf, format!("INVITE:{}", payload.invite_code), &[1]).unwrap();

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

    let db = state.db.clone();
    let cf = db.cf_handle("invites").unwrap();
    //db.get_cf(&cf, b"INVITE:{payload.invite_code}").unwrap();

    let invite_code = new_invite();
    db.put_cf(&cf, format!("INVITE:{invite_code}"), &[0])
        .unwrap();

    (StatusCode::ACCEPTED, invite_code)
}

async fn active_invites(
    State(state): State<AppState>,
    Json(payload): Json<AdminRequest>,
) -> impl IntoResponse {
    if payload.admin_token != state.admin_token {
        return (StatusCode::FORBIDDEN, "ERRROR".to_owned());
    }

    let db = state.db.clone();
    let cf = db.cf_handle("invites").unwrap();

    let iter = db.iterator_cf(&cf, IteratorMode::Start);

    let mut result = vec![];
    for item in iter {
        let (k, v) = item.unwrap();
        if v.as_ref() == &[0] {
            result.push(k);
        }
    }

    let results = result.iter().map(|f|String::from_utf8_lossy(f) + "\n").collect();
    (StatusCode::ACCEPTED, results)

}

const STRING_LEN: usize = 8;

fn new_invite() -> String {
    let random_string: String = rng()
        .sample_iter(&Alphanumeric)
        .take(STRING_LEN)
        .map(char::from) // Convert the u8 samples to chars
        .collect();
    random_string
}
