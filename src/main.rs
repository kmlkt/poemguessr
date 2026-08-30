use std::{env, str::FromStr, sync::Arc};

use axum::{
    Form, Router,
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use maud::Markup;
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::{
    AppResponse::{Html, Redir},
    common::{AppError, AppResult},
    parsing_fix::fix_linenums,
    poem_database::load_poem_db,
    problem::Problem,
    ui::{AppState, index_page},
};

mod common;
mod parsing;
mod parsing_fix;
mod poem_database;
mod problem;
mod ui;

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "Page not found"),
            err => {
                tracing::error!(error = ?err, "App error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        let page: maud::Markup = maud::html! {
            html {
                head {
                    title { (status) }
                }
                body {
                    h1 { (status) }
                    p { (message) }
                }
            }
        };

        (status, page).into_response()
    }
}

enum AppResponse {
    Redir(Redirect),
    Html(Markup),
}

impl IntoResponse for AppResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Redir(response) => response.into_response(),
            Self::Html(response) => response.into_response(),
        }
    }
}

async fn redirect_to_random_problem() -> AppResponse {
    let random_problem_seed: u64 = rand::random();
    Redir(Redirect::to(&format!("/{random_problem_seed}")))
}

#[derive(Deserialize)]
struct Answer {
    author: Option<String>,
}

async fn check_answer_and_redirect(
    Path(seed): Path<u64>,
    State(AppState { db }): State<AppState>,
    Form(Answer { author }): Form<Answer>,
) -> AppResult<AppResponse> {
    let problem = Problem::from_seed(seed, &db).await?;
    if problem.check_answer(&db, author.as_deref().unwrap_or("")) {
        Ok(redirect_to_random_problem().await)
    } else {
        Ok(Html(
            index_page(State(AppState { db }), Path(seed), Form(Answer { author })).await?,
        ))
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> AppResult<()> {
    tracing_subscriber::fmt().with_env_filter("trace").init();

    let poem_db = load_poem_db().await?;
    let port = env::var("PORT")?;
    let state = AppState {
        db: Arc::new(poem_db),
    };
    let app = Router::new()
        .route("/", get(redirect_to_random_problem))
        .route("/{seed}", get(index_page))
        .route("/{seed}", post(check_answer_and_redirect))
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;

    axum::serve(listener, app).await?;
    Ok(())
}
