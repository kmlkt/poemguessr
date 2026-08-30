use rand::{RngExt, SeedableRng, seq::IndexedRandom};

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

fn random_author(rng: &mut impl rand::Rng) -> &'static str {
    let r = rng.random_range(0..100);
    let group = match r {
        0..40 => MAJOR_AUTHORS,
        40..80 => MEDIUM_AUTHORS,
        _ => MINOR_AUTHORS,
    };
    group.choose(rng).unwrap()
}

fn random_poem(rng: &mut impl rand::Rng, db: &PoemDatabase, author: &str) -> (i32, String) {
    db.get(author).unwrap().choose(rng).unwrap().clone()
}

async fn random_poem_part(rng: &mut impl rand::Rng, poem_id: i32) -> AppResult<Vec<String>> {
    let parts_count = get_poem_parts_count(poem_id).await?;
    let part_id = rng.random_range(0..parts_count);
    let mut result = get_poem_part(poem_id, part_id).await?;

    let mut i = part_id + 1;
    while result.len() < 4 && i < parts_count {
        result.push("".into());
        result.append(&mut get_poem_part(poem_id, i).await?);
        i += 1;
    }

    i = part_id - 1;
    while result.len() < 4 && i >= 0 {
        let mut new_result = get_poem_part(poem_id, i).await?;
        new_result.push("".into());
        new_result.append(&mut result);
        result = new_result;
        i -= 1;
    }

    Ok(result)
}

fn cut_poem_part(rng: &mut impl rand::Rng, poem_part: Vec<String>) -> Vec<String> {
    if poem_part.len() > 12 {
        let start = rng.random_range(0..(poem_part.len() - 11));
        poem_part[start..(start + 12)].to_vec()
    } else {
        poem_part
    }
}

pub struct Problem {
    poem_id: i32,
    right_author: &'static str,
    pub poem: Vec<String>,
    pub authors: Vec<&'static str>,
}

pub struct SolutionComment {
    pub poem_id: i32,
    pub right: bool,
    pub right_author: &'static str,
}

impl Problem {
    pub async fn from_seed(seed: u64, db: &PoemDatabase) -> AppResult<Problem> {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
        Problem::random(&mut rng, db).await
    }
    pub async fn random(rng: &mut impl rand::Rng, db: &PoemDatabase) -> AppResult<Problem> {
        let author = random_author(rng);
        let (poem_id, _) = random_poem(rng, db, author);
        let poem_part = random_poem_part(rng, poem_id).await?;
        let poem_part_cutted = cut_poem_part(rng, poem_part);

        let mut authors = vec![author];
        while authors.len() < 4 {
            let next_author = random_author(rng);
            if !authors.contains(&next_author) {
                authors.push(next_author);
            }
        }
        authors.sort();

        Ok(Problem {
            poem_id,
            right_author: author,
            poem: poem_part_cutted,
            authors: authors,
        })
    }

    pub fn check_answer(&self, author: &str) -> SolutionComment {
        SolutionComment {
            poem_id: self.poem_id,
            right: author == self.right_author,
            right_author: self.right_author,
        }
    }
}
