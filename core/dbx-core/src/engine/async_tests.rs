#[cfg(test)]
mod tests {
    use crate::engine::async_api::DatabaseAsync;
    use crate::engine::database::Database;
    use std::sync::Arc;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_async_insert_and_get() {
        let db = Database::open_in_memory().unwrap();
        let table = "async_table";
        db.create_table(table, arrow::datatypes::Schema::empty())
            .unwrap();

        let async_db = DatabaseAsync::new(Arc::new(db));

        let key = b"async_key".to_vec();
        let val = b"async_val".to_vec();

        // 비동기 삽입
        async_db
            .insert(table.to_string(), key.clone(), val.clone())
            .await
            .unwrap();

        // 비동기 조회
        let retrieved = async_db.get(table.to_string(), key).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), val);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_async_concurrent_cas() {
        let db = Database::open_in_memory().unwrap();
        let table = "async_cas_table";
        db.create_table(table, arrow::datatypes::Schema::empty())
            .unwrap();

        let async_db = DatabaseAsync::new(Arc::new(db));
        let key = b"counter".to_vec();

        // 초기값 세팅 (0)
        async_db
            .insert(table.to_string(), key.clone(), b"0".to_vec())
            .await
            .unwrap();

        let mut handles = vec![];
        for _ in 0..10 {
            let db_clone = async_db.clone();
            let table_clone = table.to_string();
            let key_clone = key.clone();

            handles.push(tokio::spawn(async move {
                for _ in 0..10 {
                    loop {
                        let current_opt = db_clone
                            .get(table_clone.clone(), key_clone.clone())
                            .await
                            .unwrap();
                        let current = match current_opt {
                            Some(val) => val,
                            None => {
                                tokio::task::yield_now().await;
                                continue;
                            }
                        };

                        let current_str = std::str::from_utf8(&current).unwrap();
                        let current_val: i32 = current_str.parse().unwrap();

                        let next_val = current_val + 1;
                        let next_str = next_val.to_string();
                        let next_bytes = next_str.as_bytes().to_vec();

                        let success = db_clone
                            .compare_and_swap(
                                table_clone.clone(),
                                key_clone.clone(),
                                current,
                                next_bytes,
                            )
                            .await
                            .unwrap();

                        if success {
                            break;
                        }
                    }
                }
            }));
        }

        // 전체 완료 대기
        for handle in handles {
            handle.await.unwrap();
        }

        let final_opt = async_db.get(table.to_string(), key).await.unwrap();
        let final_val = final_opt.unwrap();
        let final_str = std::str::from_utf8(&final_val).unwrap();
        assert_eq!(final_str, "100", "Concurrent async CAS failed");
    }
}
