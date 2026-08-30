use crate::{common::AppResult, poem_database::load_poem_db, server::run_server};
use std::env;

mod common;
mod parsing;
mod parsing_fix;
mod poem_database;
mod problem;
mod server;
mod ui;

#[tokio::main(flavor = "current_thread")]
async fn main() -> AppResult<()> {
    let host = env::var("HOST").unwrap();
    tracing_subscriber::fmt().with_env_filter("trace").init();
    let poem_db = load_poem_db().await?;
    run_server(&host, poem_db).await?;
    Ok(())
}
