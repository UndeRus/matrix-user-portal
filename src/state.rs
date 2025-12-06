use clap::{Parser};

#[derive(Clone)]
pub struct AppState {
    pub matrix_domain: String,
    pub matrix_reg_token: String,
}

#[derive(Parser)]
pub struct Args {
    #[arg(short = 'd', long, env = "MATRIX_SERVER_DOMAIN")]
    pub matrix_domain: String,

    #[arg(short = 't', long, env = "MATRIX_REG_TOKEN")]
    pub matrix_reg_token: String,
}