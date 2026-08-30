use std::sync::Arc;

use axum::{
    Form,
    extract::{Path, State},
};
use maud::{Markup, html};

use crate::{
    common::AppResult,
    poem_database::PoemDatabase,
    problem::{self, Problem, SolutionComment},
};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<PoemDatabase>,
}

pub fn question_page(problem: Problem, answer_submit_url: &str) -> Markup {
    html! {
        p {
            @for line in problem.poem {
                (line)
                br;
            }
        }
        @for author in problem.authors {
            form action=(answer_submit_url) method="post" {
                input type="text" name="author" value=(author) hidden;
                button type="submit" {
                    (author)
                }
            }
        }
        // @match answer.author {
        //     None => {
        //     },
        //     Some(answer_author) => {
        //         @for author in problem.authors {
        //             @let color = if author == answer_author {"red"}
        //                 else if author == problem.right_author {"green"}
        //                 else {"black"};
        //             form {
        //                 button type="button" style=(format!("color: {color}")) {
        //                     (author)
        //                 }
        //             }
        //         }
        //         a href="/" {
        //             "Продолжить"
        //         }
        //     },
        // }
    }
}

pub fn answer_page(
    problem: Problem,
    user_answer: &str,
    comment: SolutionComment,
    next_problem_url: &str,
) -> Markup {
    html! {
        p {
            @for line in problem.poem {
                (line)
                br;
            }
        }
        @for author in problem.authors {
            @let color =if author == comment.right_author {"green"}
                else if author == user_answer {"red"}
                else {"black"};
            form {
                button type="button" style=(format!("color: {color}")) {
                    (author)
                }
            }
        }
        a href=(next_problem_url) {
            "Продолжить"
        }
    }
}

pub fn error_page(status: &str, comment: &str) -> Markup {
    maud::html! {
        html {
            head {
                title { (status) }
            }
            body {
                h1 { (status) }
                p { (comment) }
            }
        }
    }
}
