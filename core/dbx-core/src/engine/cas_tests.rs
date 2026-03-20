use super::*;
use crate::engine::database::Database;
use std::sync::Arc;

#[test]
fn test_insert_if_not_exists() {
    let db = Database::open_in_memory().unwrap();
    let table = "cas_test_table";
    db.create_table(table, arrow::datatypes::Schema::empty())
        .unwrap();

    let key = b"key1";
    let val1 = b"value1";
    let val2 = b"value2";

    // 첫 삽입은 성공해야 함
    let success = db.insert_if_not_exists(table, key, val1).unwrap();
    assert!(success);
    assert_eq!(db.get(table, key).unwrap().unwrap(), val1);

    // 두 번째 삽입은 실패해야 함 (이미 존재함)
    let success = db.insert_if_not_exists(table, key, val2).unwrap();
    assert!(!success);
    assert_eq!(db.get(table, key).unwrap().unwrap(), val1); // 값 변경 없음
}

#[test]
fn test_compare_and_swap() {
    let db = Database::open_in_memory().unwrap();
    let table = "cas_test_table";
    db.create_table(table, arrow::datatypes::Schema::empty())
        .unwrap();

    let key = b"key1";
    let init_val = b"init";
    let expected_val = b"init";
    let wrong_val = b"wrong";
    let new_val = b"new";

    db.insert(table, key, init_val).unwrap();

    // 예상값이 다르면 실패해야 함
    let success = db.compare_and_swap(table, key, wrong_val, new_val).unwrap();
    assert!(!success);
    assert_eq!(db.get(table, key).unwrap().unwrap(), init_val);

    // 예상값이 같으면 성공해야 함
    let success = db
        .compare_and_swap(table, key, expected_val, new_val)
        .unwrap();
    assert!(success);
    assert_eq!(db.get(table, key).unwrap().unwrap(), new_val);
}

#[test]
fn test_update_if_exists() {
    let db = Database::open_in_memory().unwrap();
    let table = "cas_test_table";
    db.create_table(table, arrow::datatypes::Schema::empty())
        .unwrap();

    let key = b"key1";
    let val1 = b"value1";
    let val2 = b"value2";

    // 존재하지 않는 키 업데이트는 실패해야 함
    let success = db.update_if_exists(table, key, val1).unwrap();
    assert!(!success);
    assert!(db.get(table, key).unwrap().is_none());

    // 키 삽입
    db.insert(table, key, val1).unwrap();

    // 존재하는 키 업데이트는 성공해야 함
    let success = db.update_if_exists(table, key, val2).unwrap();
    assert!(success);
    assert_eq!(db.get(table, key).unwrap().unwrap(), val2);
}

#[test]
fn test_delete_if_equals() {
    let db = Database::open_in_memory().unwrap();
    let table = "cas_test_table";
    db.create_table(table, arrow::datatypes::Schema::empty())
        .unwrap();

    let key = b"key1";
    let val1 = b"value1";
    let wrong_val = b"wrong";

    db.insert(table, key, val1).unwrap();

    // 값이 다르면 삭제 실패해야 함
    let success = db.delete_if_equals(table, key, wrong_val).unwrap();
    assert!(!success);
    assert!(db.get(table, key).unwrap().is_some());

    // 값이 같으면 삭제 성공해야 함
    let success = db.delete_if_equals(table, key, val1).unwrap();
    assert!(success);
    assert!(db.get(table, key).unwrap().is_none());
}

#[test]
fn test_cas_concurrency() {
    let db = Database::open_in_memory().unwrap();
    let table = "cas_test_table";
    db.create_table(table, arrow::datatypes::Schema::empty())
        .unwrap();

    let key = b"counter";
    let init_val = b"0";
    db.insert(table, key, init_val).unwrap();

    // 10개의 스레드가 동시에 counter 값을 1씩 증가시키기 경쟁 (총 10번 성공해야 함)
    // 각 스레드는 성공할 때까지 compare_and_swap 재시도
    let num_threads = 10;
    let iterations = 100;

    let db_arc = Arc::new(db);
    let mut handles = vec![];

    for _ in 0..num_threads {
        let db_clone = Arc::clone(&db_arc);
        let table_name = table.to_string();

        handles.push(std::thread::spawn(move || {
            for _ in 0..iterations {
                loop {
                    let current_opt = db_clone.get(&table_name, key).unwrap();
                    let current = match current_opt {
                        Some(val) => val,
                        None => {
                            // MVCC나 Dirty buffer 구현 방식에 의해 일시적으로 None이 반환될 수 있으므로 재시도
                            std::thread::yield_now();
                            continue;
                        }
                    };

                    let current_str = std::str::from_utf8(&current).unwrap();
                    let current_val: i32 = current_str.parse().unwrap();

                    let next_val = current_val + 1;
                    let next_str = next_val.to_string();
                    let next_bytes = next_str.as_bytes();

                    if db_clone
                        .compare_and_swap(&table_name, key, &current, next_bytes)
                        .unwrap()
                    {
                        break; // 성공하면 탈출, 실패하면 루프 재시도 (CAS 재시도 패턴)
                    }
                }
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let final_val = db_arc.get(table, key).unwrap().unwrap();
    let final_str = std::str::from_utf8(&final_val).unwrap();
    let expected = (num_threads * iterations).to_string();
    assert_eq!(final_str, expected);
}
