use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use tracing::{info, warn};

/// 스토리지 경로 관리자 (StoragePathManager)
/// 
/// 5-Tier 하이브리드 스토리지 시스템의 파편화된 파일 경로를 하나의 루트 디렉토리 산하로 통합 관리합니다.
/// 단일 폴더(Virtual File System 대체) 방식을 통해 유저의 단순 백업/복제를 지원합니다.
#[derive(Debug, Clone)]
pub struct StoragePathManager {
    root_dir: PathBuf,
}

impl StoragePathManager {
    /// 새로운 스토리지 경로 관리자를 생성합니다.
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root_dir: root.as_ref().to_path_buf(),
        }
    }

    /// 원시 루트 디렉토리
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Write-Optimized Storage (WOS) 경로 반환 및 보장
    pub fn wos_dir(&self) -> io::Result<PathBuf> {
        let p = self.root_dir.join("wos");
        fs::create_dir_all(&p)?;
        Ok(p)
    }

    /// Read-Optimized Storage (ROS / Parquet) 경로 반환 및 보장
    pub fn ros_dir(&self) -> io::Result<PathBuf> {
        let p = self.root_dir.join("ros");
        fs::create_dir_all(&p)?;
        Ok(p)
    }

    /// Erasure Coding (Cold Tier) 저장 경로 반환 및 보장
    pub fn cold_ec_dir(&self) -> io::Result<PathBuf> {
        let p = self.root_dir.join("cold_ec");
        fs::create_dir_all(&p)?;
        Ok(p)
    }

    /// Write Ahead Log 경로 (파일)
    pub fn wal_path(&self) -> PathBuf {
        self.root_dir.join("wal.log")
    }

    /// 암호화된 Write Ahead Log 경로 (파일)
    pub fn encrypted_wal_path(&self) -> PathBuf {
        self.root_dir.join("wal.enc.log")
    }

    /// Columnar Cache 분할 파일 보관 경로
    pub fn columnar_cache_dir(&self) -> io::Result<PathBuf> {
        let p = self.root_dir.join("columnar_cache");
        fs::create_dir_all(&p)?;
        Ok(p)
    }
    
    /// 메모리 초과 시 스필링할 객체 캐시(L2 Cache) 경로
    pub fn l2_cache_dir(&self) -> io::Result<PathBuf> {
        let p = self.root_dir.join("l2_cache");
        fs::create_dir_all(&p)?;
        Ok(p)
    }

    /// 임시 파일 저장소 (Plan 중간 산출물, Parquet 변환 등의 스왑 공간)
    pub fn temp_dir(&self) -> io::Result<PathBuf> {
        let p = self.root_dir.join("tmp");
        fs::create_dir_all(&p)?;
        Ok(p)
    }

    /// 데몬 기동 시 비정상 종료로 남은 고아 파일(Garbage)을 청소합니다.
    pub fn cleanup_orphans(&self) -> io::Result<()> {
        let tmp = self.root_dir.join("tmp");
        if tmp.exists() {
            info!("Cleaning up orphan temporary files in {:?}", tmp);
            // 기존 폴더를 지우고 새로 생성 (모든 안의 내용물 파기)
            fs::remove_dir_all(&tmp).ok();
            fs::create_dir_all(&tmp)?;
        }
        Ok(())
    }
}
