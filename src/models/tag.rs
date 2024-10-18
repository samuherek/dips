use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

pub type Id = String;

#[derive(Debug)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug)]
pub struct TagMeta {
    pub id: String,
    pub name: String,
}

impl Tag {
    fn new(name: &str) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now: chrono::NaiveDateTime = chrono::Utc::now().date_naive().into();
        Self {
            id,
            name: name.into(),
            created_at: now,
        }
    }
}

pub async fn get_or_create(
    tx: &mut Transaction<'_, Sqlite>,
    value: &str,
) -> Result<Id, sqlx::Error> {
    let tag_id: Option<String> = sqlx::query_scalar!("SELECT id FROM tags WHERE name = ?", value)
        .fetch_optional(&mut **tx)
        .await?;
    let tag_id = match tag_id {
        Some(id) => id,
        None => {
            let tag_new = Tag::new(value);
            sqlx::query_scalar!(
                "insert into tags (id, name, created_at) values ($1, $2, $3)",
                tag_new.id,
                tag_new.name,
                tag_new.created_at
            )
            .execute(&mut **tx)
            .await?;
            tag_new.id
        }
    };
    Ok(tag_id)
}

pub async fn create_dip_tag(
    tx: &mut Transaction<'_, Sqlite>,
    dip_id: &Uuid,
    value: &str,
) -> Result<(), sqlx::Error> {
    let tag_id = get_or_create(tx, value).await?;
    // TODO: make the UUID into a string otherwise it stores as garbage.
    sqlx::query!(
        "insert into dips_tags (dip_id, tag_id) values($1, $2)",
        dip_id,
        tag_id
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
