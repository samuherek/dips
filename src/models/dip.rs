#[derive(serde::Serialize, sqlx::FromRow, Debug)]
pub struct Dip {
    pub id: String,
    pub value: String,
    pub note: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Dip {
    pub fn new(value: &str, note: Option<&str>) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        let note = note.map(|v| v.to_string());
        Self {
            id,
            value: value.to_string(),
            note,
            created_at: now,
            updated_at: now,
        }
    }
}
