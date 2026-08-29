use std::result;

use rand::seq::IndexedRandom;

use crate::{
    common::AppResult,
    poem_database::{PoemDatabase, get_poem_part, get_poem_parts_count},
};

const MAJOR_AUTHORS: &[&str] = &[
    "Сергей Есенин",
    "Александр Блок",
    "Михаил Лермонтов",
    "Александр Пушкин",
    "Николай Некрасов",
    "Марина Цветаева",
];

const MEDIUM_AUTHORS: &[&str] = &[
    "Федор Тютчев",
    "Афанасий Фет",
    "Анна Ахматова",
    "Владимир Маяковский",
    "Аполлон Майков",
    "Иван Никитин",
    "Константин Бальмонт",
    "Осип Мандельштам",
    "Иван Крылов",
    "Василий Жуковский",
];

const MINOR_AUTHORS: &[&str] = &[
    "Николай Гоголь",
    "Валерий Брюсов",
    "Алексей Толстой",
    "Данте Алигьери",
    "Кондратий Рылеев",
    "Мольер",
    "Алексей Кольцов",
    "Петр Ершов",
    "Уильям Шекспир",
    "Александр Островский",
    "Всеволод Гаршин",
    "Иван Тургенев",
    "Аркадий Гайдар",
    "Мигель де Сервантес Сааведра",
    "Михаил Ломоносов",
    "Гомер",
    "Николай Гумилев",
    "Евгений Баратынский",
    "Александр Грибоедов",
    "Гавриил Державин",
];

fn random_author() -> &'static str {
    let r = rand::random_range(0..100);
    let group = match r {
        0..40 => MAJOR_AUTHORS,
        40..80 => MEDIUM_AUTHORS,
        _ => MINOR_AUTHORS,
    };
    group.choose(&mut rand::rng()).unwrap()
}

fn random_poem(db: &PoemDatabase, author: &str) -> (i32, String) {
    db.get(author)
        .unwrap()
        .choose(&mut rand::rng())
        .unwrap()
        .clone()
}

fn row_count(s: &str) -> usize {
    s.trim().matches("\n").count() + 1
}

fn too_small(s: &str) -> bool {
    row_count(s) < 4
}

async fn random_poem_part(poem_id: i32) -> AppResult<String> {
    let parts_count = get_poem_parts_count(poem_id).await?;
    let part_id = rand::random_range(0..parts_count);
    let mut result: String = get_poem_part(poem_id, part_id).await?;

    let mut i = part_id + 1;
    while too_small(&result) && i < parts_count {
        result += "\n\n";
        result += &get_poem_part(i, part_id).await?;
        i += 1;
    }

    i = part_id - 1;
    while too_small(&result) && i >= 0 {
        result = get_poem_part(i, part_id).await? + "\n\n" + &result;
        i -= 1;
    }

    Ok(result)
}

pub struct Problem {
    poem: String,
    authors: Vec<&'static str>,
}

impl Problem {
    pub async fn random(db: &PoemDatabase) -> AppResult<Problem> {
        let author = random_author();
        let (poem_id, _) = random_poem(db, author);
        let poem_part = random_poem_part(poem_id).await?;

        let mut authors = vec![author];
        while authors.len() < 4 {
            let next_author = random_author();
            if !authors.contains(&next_author) {
                authors.push(next_author);
            }
        }
        authors.sort();

        Ok(Problem {
            poem: poem_part,
            authors: authors,
        })
    }
}
