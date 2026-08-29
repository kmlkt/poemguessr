use markup5ever::interface::tree_builder::TreeSink;
use std::{collections::HashMap, env, fs::File, result, str::FromStr, time::Duration};
use tokio::time::error;

use axum::{Router, http::HeaderValue, routing::get};
use maud::html;
use scraper::{ElementRef, Html};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::{common::AppResult, poem_database::load_poem_db};

mod common;
mod parsing;
mod poem_database;
mod problem;

#[tokio::main(flavor = "current_thread")]
async fn main() -> AppResult<()> {
    let poem_db = load_poem_db().await?;

    for (author, poems) in poem_db {
        let count = poems.len();
        println!("{author} {count}")
    }

    return Ok(());
    tracing_subscriber::fmt().with_env_filter("trace").init();

    let database_url = env::var("DATABASE_URL")?;
    let port = env::var("PORT")?;

    let connection_options = SqliteConnectOptions::from_str(&database_url)?.create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connection_options)
        .await?;

    sqlx::migrate!().run(&pool).await?;

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    axum::serve(listener, app).await?;
    Ok(())
}
