use std::sync::Arc;

use axum::{
    Form,
    extract::{Path, State},
};
use maud::{Markup, html};
use rand::SeedableRng;

use crate::{Answer, common::AppResult, poem_database::PoemDatabase, problem::Problem};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<PoemDatabase>,
}

pub async fn index_page(
    State(AppState { db }): State<AppState>,
    Path(seed): Path<u64>,
    Form(answer): Form<Answer>,
) -> AppResult<Markup> {
    let problem = Problem::from_seed(seed, &db).await?;
    Ok(html! {
        p {
            @for line in problem.poem {
                (line)
                br;
            }
        }
        @match answer.author {
            None => {
                @for author in problem.authors {
                    form action=(format!("/{seed}")) method="post" {
                        input type="text" name="author" value=(author) hidden;
                        button type="submit" {
                            (author)
                        }
                    }
                }
            },
            Some(answer_author) => {
                @for author in problem.authors {
                    @let color = if author == answer_author {"red"}
                        else if author == problem.right_author {"green"}
                        else {"black"};
                    form {
                        button type="button" style=(format!("color: {color}")) {
                            (author)
                        }
                    }
                }
                a href="/" {
                    "Продолжить"
                }
            },
        }
    })
}
