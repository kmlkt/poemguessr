use markup5ever::interface::tree_builder::TreeSink;
use std::{env, fs::File, result, str::FromStr};

use axum::{Router, http::HeaderValue, routing::get};
use maud::html;
use scraper::ElementRef;
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
        .append("User-Agent", HeaderValue::from_static("Literaguessr"));
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

fn remove_linenums_and_references(document: scraper::Html) -> AppResult<scraper::Html> {
    let ids: Vec<_> = document
        .select(&scraper::Selector::parse(".linenum")?)
        .map(|x| x.id())
        .chain(
            document
                .select(&scraper::Selector::parse(".reference")?)
                .map(|x| x.id()),
        )
        .collect();

    let tree = scraper::HtmlTreeSink::new(document);
    for id in ids {
        tree.remove_from_parent(&id);
    }

    Ok(tree.finish())
}

struct ParsedPoem {
    title: String,
    text: String,
}

fn parse_poem(html: &str) -> AppResult<ParsedPoem> {
    let document = remove_linenums_and_references(scraper::Html::parse_document(html))?;

    let heading = document
        .select(&scraper::Selector::parse("title")?)
        .next()
        .ok_or(AppError::NotFound)?;
    let title: String = heading.text().collect();
    let title_no_author: String = title
        .split('(')
        .next()
        .ok_or(AppError::NotFound)?
        .trim()
        .into();
    let text = itertools::join(
        document
            .select(&scraper::Selector::parse(".poem>p")?)
            .map(|x| x.text())
            .flatten(),
        "",
    );
    Ok(ParsedPoem {
        title: title_no_author,
        text,
    })
}

async fn parse_all_poems(client: &reqwest::Client, list_url: &str) -> AppResult<Vec<ParsedPoem>> {
    let list_html = fetch_html(client, list_url).await?;
    let poem_urls = parse_list(&list_html)?;
    let mut result: Vec<ParsedPoem> = vec![];
    let mut i = 0;
    for poem_url in poem_urls {
        let poem_html = fetch_html(client, &format!("https://ru.wikisource.org{poem_url}")).await?;
        match parse_poem(&poem_html) {
            Ok(poem) => result.push(poem),
            Err(err) => println!("{err}{poem_url}"),
        };
        i += 1;
        if i == 3 {
            break;
        }
    }
    Ok(result)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> AppResult<()> {
    let client = reqwest::Client::new();
    //     let html = fetch_html(&client,
    //         //"https://ru.wikisource.org/wiki/%D0%A1%D1%82%D0%B8%D1%85%D0%BE%D1%82%D0%B2%D0%BE%D1%80%D0%B5%D0%BD%D0%B8%D1%8F_%D0%9F%D1%83%D1%88%D0%BA%D0%B8%D0%BD%D0%B0_1809%E2%80%941825",
    // "https://ru.wikisource.org/wiki/%D0%9B%D0%B8%D1%86%D0%B8%D0%BD%D0%B8%D1%8E_(%D0%9F%D1%83%D1%88%D0%BA%D0%B8%D0%BD)"
    //     ).await?;
    //     //println!("{html}");
    //     parse_poem(&html)?;

    let poems = parse_all_poems(
        &client,
        "https://ru.wikisource.org/wiki/%D0%A1%D1%82%D0%B8%D1%85%D0%BE%D1%82%D0%B2%D0%BE%D1%80%D0%B5%D0%BD%D0%B8%D1%8F_%D0%9F%D1%83%D1%88%D0%BA%D0%B8%D0%BD%D0%B0_1809%E2%80%941825",
    ).await?;

    for poem in poems {
        let title = poem.title;
        let text = poem.text;
        println!("{title}");
        println!("{text}");
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
