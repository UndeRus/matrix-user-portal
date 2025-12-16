use clap::{Parser};

use crate::db::InvitesRepo;

#[derive(Clone)]
pub struct AppState {
    pub matrix_server_url: String,
    pub matrix_reg_token: String,
    pub repo: InvitesRepo,
    pub admin_token: String,
}

#[derive(Parser)]
pub struct Args {
    #[arg(short = 'u', long, env = "MATRIX_SERVER_URL")]
    pub matrix_server_url: String,

    #[arg(short = 't', long, env = "MATRIX_REG_TOKEN")]
    pub matrix_reg_token: String,

    #[arg(short ='a', long, env = "ADMIN_TOKEN")]
    pub admin_token: String,
}