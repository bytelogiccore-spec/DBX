//! Versionable 트레이트 — 범용 MVCC 버전 관리를 위한 트레이트
//!
//! 임의의 타입에 버전 관리 기능을 부여하여 MVCC 프레임워크에서 사용할 수 있도록 합니다.

use crate::error::{DbxError, DbxResult};

/// 버전 관리가 가능한 타입을 정의하는 트레이트
///
/// 이 트레이트를 구현하면 `VersionManager`를 통해 해당 타입의 버전을 관리할 수 있습니다.
///
/// # Example
///
/// ```rust
/// use dbx_core::transaction::versionable::Versionable;
/// use dbx_core::error::DbxResult;
///
/// #[derive(Clone, Debug, PartialEq)]
/// struct User {
///     id: u64,
///     name: String,
/// }
///
/// impl Versionable for User {
///     fn version_key(&self) -> Vec<u8> {
///         self.id.to_be_bytes().to_vec()
///     }
///
///     fn serialize(&self) -> DbxResult<Vec<u8>> {
///         let mut bytes = Vec::new();
///         bytes.extend_from_slice(&self.id.to_be_bytes());
///         bytes.extend_from_slice(self.name.as_bytes());
///         Ok(bytes)
///     }
///
///     fn deserialize(bytes: &[u8]) -> DbxResult<Self> {
///         if bytes.len() < 8 {
///             return Err(dbx_core::error::DbxError::Storage("Invalid User data".into()));
///         }
///         let id = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
///         let name = String::from_utf8(bytes[8..].to_vec())
///             .map_err(|e| dbx_core::error::DbxError::Storage(e.to_string()))?;
///         Ok(User { id, name })
///     }
/// }
/// ```
pub trait Versionable: Sized + Clone {
    /// 이 객체의 버전 키를 생성합니다.
    ///
    /// 버전 키는 동일한 논리적 엔티티의 여러 버전을 식별하는 데 사용됩니다.
    /// 예: User 엔티티의 경우 user_id를 버전 키로 사용할 수 있습니다.
    fn version_key(&self) -> Vec<u8>;

    /// 이 객체를 바이트 배열로 직렬화합니다.
    fn serialize(&self) -> DbxResult<Vec<u8>>;

    /// 바이트 배열에서 객체를 역직렬화합니다.
    fn deserialize(bytes: &[u8]) -> DbxResult<Self>;
}

// 기본 타입에 대한 Versionable 구현

impl Versionable for Vec<u8> {
    fn version_key(&self) -> Vec<u8> {
        self.clone()
    }

    fn serialize(&self) -> DbxResult<Vec<u8>> {
        Ok(self.clone())
    }

    fn deserialize(bytes: &[u8]) -> DbxResult<Self> {
        Ok(bytes.to_vec())
    }
}

impl Versionable for String {
    fn version_key(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn serialize(&self) -> DbxResult<Vec<u8>> {
        Ok(self.as_bytes().to_vec())
    }

    fn deserialize(bytes: &[u8]) -> DbxResult<Self> {
        String::from_utf8(bytes.to_vec())
            .map_err(|e| DbxError::Storage(format!("UTF-8 decode error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct TestUser {
        id: u64,
        name: String,
        age: u32,
    }

    impl Versionable for TestUser {
        fn version_key(&self) -> Vec<u8> {
            self.id.to_be_bytes().to_vec()
        }

        fn serialize(&self) -> DbxResult<Vec<u8>> {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(&self.id.to_be_bytes());
            bytes.extend_from_slice(&self.age.to_be_bytes());
            let name_bytes = self.name.as_bytes();
            bytes.extend_from_slice(&(name_bytes.len() as u32).to_be_bytes());
            bytes.extend_from_slice(name_bytes);
            Ok(bytes)
        }

        fn deserialize(bytes: &[u8]) -> DbxResult<Self> {
            if bytes.len() < 16 {
                return Err(DbxError::Storage("Invalid TestUser data".into()));
            }
            let id = u64::from_be_bytes(bytes[0..8].try_into().unwrap());
            let age = u32::from_be_bytes(bytes[8..12].try_into().unwrap());
            let name_len = u32::from_be_bytes(bytes[12..16].try_into().unwrap()) as usize;
            if bytes.len() < 16 + name_len {
                return Err(DbxError::Storage("Invalid TestUser name length".into()));
            }
            let name = String::from_utf8(bytes[16..16 + name_len].to_vec())
                .map_err(|e| DbxError::Storage(e.to_string()))?;
            Ok(TestUser { id, name, age })
        }
    }

    #[test]
    fn test_versionable_basic() {
        // Vec<u8> 기본 구현 테스트
        let data = vec![1, 2, 3, 4, 5];
        let key = data.version_key();
        assert_eq!(key, data);

        let serialized = data.serialize().unwrap();
        assert_eq!(serialized, data);

        let deserialized = Vec::<u8>::deserialize(&serialized).unwrap();
        assert_eq!(deserialized, data);
    }

    #[test]
    fn test_versionable_custom_types() {
        // 커스텀 타입 TestUser 테스트
        let user = TestUser {
            id: 42,
            name: "Alice".to_string(),
            age: 30,
        };

        let key = user.version_key();
        assert_eq!(key, 42u64.to_be_bytes().to_vec());

        let serialized = user.serialize().unwrap();
        let deserialized = TestUser::deserialize(&serialized).unwrap();
        assert_eq!(deserialized, user);
    }

    #[test]
    fn test_versionable_serialization_roundtrip() {
        // String 직렬화/역직렬화 왕복 테스트
        let original = "Hello, MVCC!".to_string();
        let serialized = original.serialize().unwrap();
        let deserialized = String::deserialize(&serialized).unwrap();
        assert_eq!(deserialized, original);

        // TestUser 왕복 테스트
        let user = TestUser {
            id: 100,
            name: "Bob".to_string(),
            age: 25,
        };
        let serialized = user.serialize().unwrap();
        let deserialized = TestUser::deserialize(&serialized).unwrap();
        assert_eq!(deserialized, user);
    }

    #[test]
    fn test_versionable_version_key_uniqueness() {
        // 동일한 ID를 가진 사용자는 동일한 버전 키를 가져야 함
        let user1 = TestUser {
            id: 1,
            name: "Alice".to_string(),
            age: 30,
        };
        let user2 = TestUser {
            id: 1,
            name: "Alice Updated".to_string(),
            age: 31,
        };
        assert_eq!(user1.version_key(), user2.version_key());

        // 다른 ID를 가진 사용자는 다른 버전 키를 가져야 함
        let user3 = TestUser {
            id: 2,
            name: "Bob".to_string(),
            age: 25,
        };
        assert_ne!(user1.version_key(), user3.version_key());
    }

    #[test]
    fn test_versionable_with_complex_data() {
        // 복잡한 데이터 구조 처리 테스트
        #[derive(Clone, Debug, PartialEq)]
        struct ComplexData {
            id: u64,
            tags: Vec<String>,
            metadata: Vec<(String, String)>,
        }

        impl Versionable for ComplexData {
            fn version_key(&self) -> Vec<u8> {
                self.id.to_be_bytes().to_vec()
            }

            fn serialize(&self) -> DbxResult<Vec<u8>> {
                let mut bytes = Vec::new();
                bytes.extend_from_slice(&self.id.to_be_bytes());

                // tags 직렬화
                bytes.extend_from_slice(&(self.tags.len() as u32).to_be_bytes());
                for tag in &self.tags {
                    let tag_bytes = tag.as_bytes();
                    bytes.extend_from_slice(&(tag_bytes.len() as u32).to_be_bytes());
                    bytes.extend_from_slice(tag_bytes);
                }

                // metadata 직렬화
                bytes.extend_from_slice(&(self.metadata.len() as u32).to_be_bytes());
                for (k, v) in &self.metadata {
                    let k_bytes = k.as_bytes();
                    let v_bytes = v.as_bytes();
                    bytes.extend_from_slice(&(k_bytes.len() as u32).to_be_bytes());
                    bytes.extend_from_slice(k_bytes);
                    bytes.extend_from_slice(&(v_bytes.len() as u32).to_be_bytes());
                    bytes.extend_from_slice(v_bytes);
                }

                Ok(bytes)
            }

            fn deserialize(bytes: &[u8]) -> DbxResult<Self> {
                if bytes.len() < 8 {
                    return Err(DbxError::Storage("Invalid ComplexData".into()));
                }

                let mut offset = 0;
                let id = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
                offset += 8;

                // tags 역직렬화
                if bytes.len() < offset + 4 {
                    return Err(DbxError::Storage("Invalid tags count".into()));
                }
                let tags_count = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
                offset += 4;

                let mut tags = Vec::new();
                for _ in 0..tags_count {
                    if bytes.len() < offset + 4 {
                        return Err(DbxError::Storage("Invalid tag length".into()));
                    }
                    let tag_len = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
                    offset += 4;
                    if bytes.len() < offset + tag_len {
                        return Err(DbxError::Storage("Invalid tag data".into()));
                    }
                    let tag = String::from_utf8(bytes[offset..offset + tag_len].to_vec())
                        .map_err(|e| DbxError::Storage(e.to_string()))?;
                    tags.push(tag);
                    offset += tag_len;
                }

                // metadata 역직렬화
                if bytes.len() < offset + 4 {
                    return Err(DbxError::Storage("Invalid metadata count".into()));
                }
                let metadata_count = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
                offset += 4;

                let mut metadata = Vec::new();
                for _ in 0..metadata_count {
                    if bytes.len() < offset + 4 {
                        return Err(DbxError::Storage("Invalid metadata key length".into()));
                    }
                    let k_len = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
                    offset += 4;
                    if bytes.len() < offset + k_len {
                        return Err(DbxError::Storage("Invalid metadata key data".into()));
                    }
                    let k = String::from_utf8(bytes[offset..offset + k_len].to_vec())
                        .map_err(|e| DbxError::Storage(e.to_string()))?;
                    offset += k_len;

                    if bytes.len() < offset + 4 {
                        return Err(DbxError::Storage("Invalid metadata value length".into()));
                    }
                    let v_len = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
                    offset += 4;
                    if bytes.len() < offset + v_len {
                        return Err(DbxError::Storage("Invalid metadata value data".into()));
                    }
                    let v = String::from_utf8(bytes[offset..offset + v_len].to_vec())
                        .map_err(|e| DbxError::Storage(e.to_string()))?;
                    offset += v_len;

                    metadata.push((k, v));
                }

                Ok(ComplexData { id, tags, metadata })
            }
        }

        let data = ComplexData {
            id: 999,
            tags: vec!["rust".to_string(), "database".to_string(), "mvcc".to_string()],
            metadata: vec![
                ("author".to_string(), "Team C".to_string()),
                ("version".to_string(), "1.0".to_string()),
            ],
        };

        let serialized = data.serialize().unwrap();
        let deserialized = ComplexData::deserialize(&serialized).unwrap();
        assert_eq!(deserialized, data);
        assert_eq!(deserialized.version_key(), 999u64.to_be_bytes().to_vec());
    }
}
