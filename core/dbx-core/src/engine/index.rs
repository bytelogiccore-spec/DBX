//! Index API — Hash Index operations

use crate::engine::Database;
use crate::error::DbxResult;

impl Database {
    /// 인덱스를 생성합니다.
    ///
    /// 지정된 테이블의 컬럼에 Hash Index를 생성합니다.
    /// O(1) 조회 성능을 제공합니다.
    ///
    /// # 인자
    ///
    /// * `table` - 테이블 이름
    /// * `column` - 인덱스를 생성할 컬럼 이름
    ///
    /// # 예제
    ///
    /// ```rust
    /// # use dbx_core::Database;
    /// # fn main() -> dbx_core::DbxResult<()> {
    /// let db = Database::open_in_memory()?;
    /// db.create_index("users", "id")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_index(&self, table: &str, column: &str) -> DbxResult<()> {
        self.index.create_index(table, column)
    }

    /// 인덱스를 삭제합니다.
    ///
    /// # 인자
    ///
    /// * `table` - 테이블 이름
    /// * `column` - 컬럼 이름
    pub fn drop_index(&self, table: &str, column: &str) -> DbxResult<()> {
        self.index.drop_index(table, column)
    }

    /// 인덱스를 사용하여 행 ID를 조회합니다.
    ///
    /// O(1) 시간 복잡도로 조회합니다.
    ///
    /// # 인자
    ///
    /// * `table` - 테이블 이름
    /// * `column` - 컬럼 이름
    /// * `value` - 조회할 값
    ///
    /// # 반환
    ///
    /// 해당 값을 가진 행 ID 목록
    pub fn index_lookup(&self, table: &str, column: &str, value: &[u8]) -> DbxResult<Vec<usize>> {
        self.index.lookup(table, column, value)
    }

    /// 인덱스가 존재하는지 확인합니다.
    pub fn has_index(&self, table: &str, column: &str) -> bool {
        self.index.has_index(table, column)
    }
}
