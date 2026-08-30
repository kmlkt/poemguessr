use std::collections::HashMap;

use crate::common::AppResult;

pub type PoemDatabase = HashMap<String, Vec<(i32, String)>>;

pub async fn load_poem_db() -> AppResult<PoemDatabase> {
    Ok(serde_json::from_slice(
        &tokio::fs::read("poem_db.json").await?,
    )?)
}

pub async fn save_poem_db(poem_db: &PoemDatabase) -> AppResult<()> {
    tokio::fs::write("poem_db.json", serde_json::to_string(poem_db).unwrap()).await?;
    Ok(())
}

fn poem_part_path(id: i32, part_id: i32) -> String {
    return format!("poem_db/{id}/{part_id}");
}

pub async fn save_poem(id: i32, text: &[Vec<String>]) -> AppResult<()> {
    tokio::fs::create_dir_all(format!("poem_db/{id}")).await?;
    for (part_id, paragraph) in text.iter().enumerate() {
        tokio::fs::write(
            poem_part_path(id, part_id.try_into().unwrap()),
            paragraph.join("\n"),
        )
        .await?;
    }
    Ok(())
}

pub async fn get_poem_parts_count(id: i32) -> AppResult<i32> {
    for part_id in 0..100000 {
        if !tokio::fs::try_exists(poem_part_path(id, part_id)).await? {
            return Ok(part_id);
        }
    }
    Ok(100000)
}

pub async fn get_poem_part(id: i32, part_id: i32) -> AppResult<Vec<String>> {
    tracing::trace!(id, part_id, "get_poem_part");
    Ok(tokio::fs::read_to_string(poem_part_path(id, part_id))
        .await?
        .lines()
        .map(|x| x.into())
        .collect())
}
