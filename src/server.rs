use std::sync::Arc;

use axum::{
    Form, Router,
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::{
    common::{AppError, AppResult},
    poem_database::PoemDatabase,
    problem::Problem,
    ui::{AppState, answer_page, error_page, question_page},
};

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "Page not found"),
            err => {
                tracing::error!(error = ?err, "App error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        let page = error_page(status.as_str(), message);

        (status, page).into_response()
    }
}

const INDEX_URL: &str = "/";
const PROBLEM_URL_TEMPLATE: &str = "/{seed}";

pub fn problem_url(seed: u64) -> String {
    format!("/{seed}")
}

pub fn random_problem_url() -> String {
    problem_url(rand::random())
}

#[derive(Deserialize)]
struct AnswerForm {
    author: String,
}

pub async fn run_server(host: &str, poem_db: PoemDatabase) -> AppResult<()> {
    let state = AppState {
        db: Arc::new(poem_db),
    };
    let app = Router::new()
        .route(INDEX_URL, get(async || Redirect::to(&random_problem_url())))
        .route(
            PROBLEM_URL_TEMPLATE,
            get(async |State(AppState { db }), Path(seed)| -> AppResult<_> {
                Ok(question_page(
                    Problem::from_seed(seed, &db).await?,
                    &problem_url(seed),
                ))
            }),
        )
        .route(
            PROBLEM_URL_TEMPLATE,
            post(
                async |State(AppState { db }),
                       Path(seed),
                       Form(AnswerForm { author })|
                       -> AppResult<_> {
                    let problem = Problem::from_seed(seed, &db).await?;
                    let comment = problem.check_answer(&author);
                    Ok(answer_page(
                        problem,
                        &author,
                        comment,
                        &random_problem_url(),
                    ))
                },
            ),
        )
        //.route(PROBLEM_URL_TEMPLATE, post(check_answer_and_redirect))
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(host).await?;

    axum::serve(listener, app).await?;
    Ok(())
}
