#[derive(serde::Serialize, sqlx::FromRow, Debug)]
pub struct Dip {
    pub id: String,
    pub value: String,
    pub note: Option<String>,
    pub dir_context_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Dip {
    pub fn new(context_id: &str, value: &str, note: Option<&str>) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        let note = note.map(|v| v.to_string());
        Self {
            id,
            value: value.into(),
            note,
            dir_context_id: context_id.into(),
            created_at: now,
            updated_at: now,
        }
    }
}
