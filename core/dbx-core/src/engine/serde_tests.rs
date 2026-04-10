#[cfg(test)]
mod tests {
    use crate::engine::database::Database;
    use crate::traits::DatabaseSerde;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct UserProfile {
        id: u64,
        username: String,
        email: String,
        is_active: bool,
    }

    #[test]
    fn test_serde_insert_get() {
        let db = Database::open_in_memory().unwrap();
        let table = "users";
        db.create_table(table, arrow::datatypes::Schema::empty())
            .unwrap();

        let profile = UserProfile {
            id: 12345,
            username: "dbx_hero".to_string(),
            email: "hero@bytelogic.com".to_string(),
            is_active: true,
        };

        // Struct 삽입
        let key = b"user:12345";
        db.insert_struct(table, key, &profile).unwrap();

        // Struct 조회 및 역직렬화
        let retrieved: Option<UserProfile> = db.get_struct(table, key).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), profile);

        // 없는 키 조회
        let not_found: Option<UserProfile> = db.get_struct(table, b"user:99999").unwrap();
        assert!(not_found.is_none());
    }
}
