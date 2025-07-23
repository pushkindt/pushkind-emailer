use chrono::NaiveDateTime;

pub struct Recipient {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub hub_id: i32,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub unsubscribed_at: Option<NaiveDateTime>,
}
