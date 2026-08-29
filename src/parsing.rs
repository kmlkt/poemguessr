use markup5ever::interface::tree_builder::TreeSink;
use std::{collections::HashMap, env, fs::File, result, str::FromStr, time::Duration};

use axum::http::HeaderValue;
use scraper::{ElementRef, Html};

use crate::{
    common::{AppError, AppResult},
    poem_database::{PoemDatabase, load_poem_db, save_poem, save_poem_db},
};

async fn fetch_html(client: &reqwest::Client, url: &str) -> AppResult<String> {
    let mut request = reqwest::Request::new(reqwest::Method::GET, reqwest::Url::parse(url)?);
    request
        .headers_mut()
        .append("User-Agent", HeaderValue::from_static("Poemguessr"));
    request.timeout_mut().replace(Duration::from_millis(1000));
    Ok(client.execute(request).await?.text().await?)
}

fn parse_list(html: &str) -> AppResult<Vec<String>> {
    let document = scraper::Html::parse_document(html);
    let list = document
        .select(&scraper::Selector::parse(".mw-parser-output")?)
        .next()
        .ok_or(AppError::NotFound)?;
    let link_selector = &scraper::Selector::parse("li>a")?;
    let links = list.select(link_selector);
    Ok(links
        .filter(|x| !x.value().attr("class").unwrap_or("").contains("external"))
        .map(|x| x.attr("href").unwrap_or("").into())
        .collect())
}

struct Poem {
    title: String,
    author: String,
    text: Vec<Vec<String>>,
}

impl Poem {
    async fn fetch(id: i32, client: &reqwest::Client) -> AppResult<Poem> {
        let mut parts = Vec::new();
        for part_id in 1..100 {
            match PoemPart::fetch(client, id, part_id).await {
                Ok(part) => {
                    if (part_id == 1 && part.text.len() == 0) || !part.next_exists {
                        parts.push(part);
                        break;
                    } else {
                        parts.push(part);
                    }
                }
                Err(AppError::Reqwest(err)) => {
                    if part_id == 1 {
                        return Err(err.into());
                    } else {
                        break;
                    }
                }
                Err(err) => return Err(err),
            }
        }
        Ok(Poem::combine(parts))
    }

    fn combine(parts: Vec<PoemPart>) -> Poem {
        let title = parts
            .iter()
            .find_map(|x| x.title.as_ref())
            .cloned()
            .map(|x| x.trim().into())
            .unwrap_or_else(|| "***".into());
        let author = parts
            .iter()
            .find_map(|x| x.author.as_ref())
            .cloned()
            .map(|x| x.trim().into())
            .unwrap_or_else(|| "noname".into());
        let text = parts.into_iter().flat_map(|x| x.text).collect();
        Poem {
            title,
            author,
            text,
        }
    }
}

struct PoemPart {
    title: Option<String>,
    author: Option<String>,
    text: Vec<Vec<String>>,
    next_exists: bool,
}

impl PoemPart {
    async fn fetch(client: &reqwest::Client, id: i32, part_id: i32) -> AppResult<PoemPart> {
        println!("{id} {part_id}");
        Ok(PoemPart::parse(
            &fetch_html(
                client,
                &format!("https://ilibrary.ru/text/{id}/p.{part_id}/index.html"),
            )
            .await?,
        ))
    }

    fn parse(html: &str) -> PoemPart {
        let document = scraper::Html::parse_document(html);

        let title = document
            .select(&scraper::Selector::parse(".title").unwrap())
            .next()
            .map(|x| x.text().collect());

        let author = document
            .select(&scraper::Selector::parse(".author").unwrap())
            .next()
            .map(|x| x.text().collect());

        let text = document
            .select(&scraper::Selector::parse("#text>#pmt1").unwrap())
            .next()
            .map(|x| {
                x.select(&scraper::Selector::parse("z").unwrap())
                    .map(|z| {
                        z.select(&scraper::Selector::parse("v").unwrap())
                            .map(|v| v.text().collect::<String>())
                            .collect::<Vec<String>>()
                    })
                    .collect::<Vec<Vec<String>>>()
            })
            .unwrap_or(vec![]);

        let next_exists = document
            .select(&scraper::Selector::parse(".je").unwrap())
            .next()
            .is_some();

        PoemPart {
            title,
            author,
            text,
            next_exists,
        }
    }
}

pub async fn parse_everything() -> AppResult<()> {
    let mut poem_db: PoemDatabase = load_poem_db().await?;

    let client = reqwest::Client::new();
    for poem_id in 1694..10000 {
        match Poem::fetch(poem_id, &client).await {
            Ok(Poem {
                title,
                author,
                text,
            }) => {
                {
                    println!("{title:?}");
                    println!("{author:?}");
                }
                if text.len() == 0 {
                    continue;
                }

                if !poem_db.contains_key(&author) {
                    poem_db.insert(author.clone(), vec![]);
                }
                poem_db
                    .get_mut(&author)
                    .unwrap()
                    .push((poem_id, title.clone()));
                save_poem(poem_id, &text).await?;
            }
            Err(err) => println!("{err:?}"),
        }
        if poem_id % 10 == 0 {
            save_poem_db(&poem_db).await?;
        }
    }
    save_poem_db(&poem_db).await?;
    Ok(())
}
