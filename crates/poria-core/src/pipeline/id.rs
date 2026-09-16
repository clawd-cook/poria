use chrono::Utc;

pub fn create_pipeline_id() -> String {
    let date = Utc::now().format("%Y%m%d").to_string();
    let suffix = nanoid::nanoid!(8);
    format!("pl-{}-{}", date, suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_id_format() {
        let id = create_pipeline_id();
        assert!(id.starts_with("pl-"));
        let parts: Vec<&str> = id.splitn(3, '-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 8); // YYYYMMDD
        assert_eq!(parts[2].len(), 8); // nanoid
    }
}
