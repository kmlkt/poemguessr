use markup5ever::interface::tree_builder::TreeSink;
use std::{collections::HashMap, env, fs::File, result, str::FromStr, time::Duration};
use tokio::time::error;

use axum::{Router, http::HeaderValue, routing::get};
use maud::html;
use scraper::{ElementRef, Html};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("not found")]
    NotFound,

    #[error(transparent)]
    Var(#[from] env::VarError),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    ReqwestParse(#[from] url::ParseError),

    #[error("scraper error: {0}")]
    Scraper(String),
}

impl<'a> From<scraper::error::SelectorErrorKind<'a>> for AppError {
    fn from(error: scraper::error::SelectorErrorKind<'a>) -> Self {
        Self::Scraper(error.to_string())
    }
}

type AppResult<T> = Result<T, AppError>;

fn smth() -> AppResult<i32> {
    let x = File::open("")?;
    Ok(1)
}

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

type PoemDatabase = HashMap<String, Vec<(i32, String)>>;

struct Poem {
    id: i32,
    title: String,
    author: String,
    text: Vec<Vec<String>>,
}

impl Poem {
    async fn fetch(client: &reqwest::Client, id: i32) -> AppResult<Poem> {
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
        Ok(Poem::combine(id, parts))
    }

    fn combine(id: i32, parts: Vec<PoemPart>) -> Poem {
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
            id,
            title,
            author,
            text,
        }
    }

    async fn save(&self) -> AppResult<()> {
        let id = self.id;
        tokio::fs::create_dir_all(format!("poem_db/{id}")).await?;
        for (i, paragraph) in self.text.iter().enumerate() {
            tokio::fs::write(format!("poem_db/{id}/{i}"), paragraph.join("\n")).await?;
        }
        Ok(())
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

// async fn parse_all_poems(client: &reqwest::Client, list_url: &str) -> AppResult<Vec<ParsedPoem>> {
//     let list_html = fetch_html(client, list_url).await?;
//     let poem_urls = parse_list(&list_html)?;
//     let mut result: Vec<ParsedPoem> = vec![];
//     let mut i = 0;
//     for poem_url in poem_urls {
//         let poem_html = fetch_html(client, &format!("https://ru.wikisource.org{poem_url}")).await?;
//         match parse_page(&poem_html) {
//             Ok(poem) => result.push(poem),
//             Err(err) => println!("{err}{poem_url}"),
//         };
//         i += 1;
//         if i == 3 {
//             break;
//         }
//     }
//     Ok(result)
// }

#[tokio::main(flavor = "current_thread")]
async fn main() -> AppResult<()> {
    let mut poem_db: PoemDatabase =
        serde_json::from_slice(&tokio::fs::read("poem_db.json").await?).unwrap();

    for (author, poems) in poem_db {
        let count = poems.len();
        println!("{author} {count}")
    }

    // let client = reqwest::Client::new();
    // for poem_id in 1694..10000 {
    //     match Poem::fetch(&client, poem_id).await {
    //         Ok(poem) => {
    //             {
    //                 let title = &poem.title;
    //                 let author = &poem.author;
    //                 println!("{title:?}");
    //                 println!("{author:?}");
    //             }
    //             if poem.text.len() == 0 {
    //                 continue;
    //             }

    //             if !poem_db.contains_key(&poem.author) {
    //                 poem_db.insert(poem.author.clone(), vec![]);
    //             }
    //             poem_db
    //                 .get_mut(&poem.author)
    //                 .unwrap()
    //                 .push((poem_id, poem.title.clone()));
    //             poem.save().await?;
    //         }
    //         Err(err) => println!("{err:?}"),
    //     }
    //     if poem_id % 10 == 0 {
    //         tokio::fs::write("poem_db.json", serde_json::to_string(&poem_db).unwrap()).await?;
    //     }
    // }

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
