use crate::{
    common::AppResult,
    poem_database::{get_poem_part, get_poem_parts_count, save_poem},
};

pub async fn fix_linenums(poem_id: i32) -> AppResult<()> {
    let parts_count = get_poem_parts_count(poem_id).await?;
    let mut parts: Vec<Vec<String>> = vec![];
    for part_id in 0..parts_count {
        let part = get_poem_part(poem_id, part_id).await?;
        let part_fixed = part
            .iter()
            .map(|x| {
                x.trim_start_matches(&['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'])
                    .into()
            })
            .collect();
        parts.push(part_fixed);
    }
    save_poem(poem_id, &parts).await?;
    Ok(())
}
