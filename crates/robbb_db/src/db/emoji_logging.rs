use anyhow::{Context, Result};

use super::Db;

use serenity::model::{id::EmojiId, misc::EmojiIdentifier};

pub struct EmojiStats {
    pub emoji: EmojiIdentifier,
    pub reactions: u64,
    pub in_text: u64,
}

impl EmojiStats {
    pub fn new(emoji_id: EmojiIdentifier) -> EmojiStats {
        EmojiStats { emoji: emoji_id, reactions: 0, in_text: 0 }
    }
}

pub enum Ordering {
    Ascending,
    Descending,
}

impl Db {
    #[tracing::instrument(skip_all)]
    pub async fn alter_emoji_reaction_count(
        &self,
        amount: i64,
        emoji: &EmojiIdentifier,
    ) -> Result<EmojiStats> {
        let mut conn = self.pool.acquire().await?;
        let emoji_str = &emoji.name;
        let id = emoji.id.0 as i64;
        sqlx::query!("insert into emoji_stats (emoji_id, emoji_name, reaction_usage, animated) values (?1, ?2, max(0, ?3), ?4) on conflict(emoji_id) do update set reaction_usage=max(0, reaction_usage + ?3)",
            id, emoji_str, amount, emoji.animated)
            .execute(&mut conn)
            .await?;
        self.get_emoji_usage_by_id(emoji).await
    }

    #[tracing::instrument(skip_all)]
    pub async fn alter_emoji_text_count(
        &self,
        amount: i64,
        emoji: &EmojiIdentifier,
    ) -> Result<EmojiStats> {
        let mut conn = self.pool.acquire().await?;
        let id = emoji.id.0 as i64;
        let emoji_str = &emoji.name;
        sqlx::query!("insert into emoji_stats (emoji_id, emoji_name, in_text_usage, animated) values (?1, ?2, max(0, ?3), ?4) on conflict(emoji_id) do update set in_text_usage=max(0, in_text_usage + ?3)",
            id, emoji_str, amount, emoji.animated)
            .execute(&mut conn)
            .await?;
        self.get_emoji_usage_by_id(emoji).await
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_emoji_usage_by_id(&self, emoji: &EmojiIdentifier) -> Result<EmojiStats> {
        let mut conn = self.pool.acquire().await?;
        let emoji_id = emoji.id.0 as i64;
        let value = sqlx::query!("select * from emoji_stats where emoji_id=?", emoji_id)
            .fetch_optional(&mut conn)
            .await?;
        Ok(value
            .map(|x| EmojiStats {
                emoji: EmojiIdentifier {
                    id: EmojiId(x.emoji_id as u64),
                    animated: x.animated != 0,
                    name: x.emoji_name.unwrap(),
                },
                in_text: x.in_text_usage as u64,
                reactions: x.reaction_usage as u64,
            })
            .unwrap_or_else(|| EmojiStats::new(emoji.clone())))
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_emoji_usage_by_name(&self, emoji: &str) -> Result<EmojiStats> {
        let mut conn = self.pool.acquire().await?;
        let value = sqlx::query!("select * from emoji_stats where emoji_name=?", emoji)
            .fetch_optional(&mut conn)
            .await?;
        value
            .map(|x| EmojiStats {
                emoji: EmojiIdentifier {
                    id: EmojiId(x.emoji_id as u64),
                    animated: x.animated != 0,
                    name: x.emoji_name.unwrap(),
                },
                in_text: x.in_text_usage as u64,
                reactions: x.reaction_usage as u64,
            })
            .context("Could not find emoji by that name")
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_top_emoji_stats(
        &self,
        count: u16,
        ordering: Ordering,
    ) -> Result<Vec<EmojiStats>> {
        // This exists to allow generic creation of queries, as the queries are two distinct types
        // and cannot be used in a match without also constructing the struct
        macro_rules! process_emoji_stats_query {
            ($query:expr,$limit:tt) => {{
                let mut conn = self.pool.acquire().await?;
                let records = sqlx::query!($query, $limit).fetch_all(&mut conn).await?;

                Ok(records
                    .into_iter()
                    .map(|x| EmojiStats {
                        emoji: EmojiIdentifier {
                            id: EmojiId(x.emoji_id as u64),
                            animated: x.animated != 0,
                            name: x.emoji_name.unwrap(),
                        },
                        in_text: x.in_text_usage as u64,
                        reactions: x.reaction_usage as u64,
                    })
                    .collect())
            }};
        }
        match ordering {
            Ordering::Ascending => process_emoji_stats_query!(
                r#"select *, in_text_usage + reaction_usage as "usage!: i32" FROM emoji_stats order by "usage!: i32" ASC limit ?"#,
                count
            ),
            Ordering::Descending => process_emoji_stats_query!(
                r#"select *, in_text_usage + reaction_usage as "usage!: i32" FROM emoji_stats order by "usage!: i32" DESC limit ?"#,
                count
            ),
        }
    }
}
