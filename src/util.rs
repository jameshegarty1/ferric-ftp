use chrono::{DateTime, Utc};
pub fn generate_first_id() -> u32 {
    let utc: DateTime<Utc> = Utc::now();
    utc.timestamp() as u32
}

