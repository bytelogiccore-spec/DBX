//! Job DAG — DAG 기반 작업 의존성 스케줄러
//!
//! Kahn's algorithm 위상 정렬을 사용하여 작업 실행 순서를 결정하고
//! 순환 의존성을 감지합니다.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::error::{DbxError, DbxResult};

/// 작업 스케줄 타입
#[derive(Debug, Clone)]
pub enum JobSchedule {
    /// 고정 주기 실행 (seconds)
    Interval(std::time::Duration),
    /// 다른 작업 완료 후 실행
    After(String),
    /// 수동 트리거
    Manual,
}

/// DAG 기반 작업 의존성 스케줄러
///
/// - `add_job`: 작업 등록
/// - `add_dependency`: A → B 의존성 추가 (B는 A 완료 후 실행)
/// - `resolve_execution_order`: Kahn's algorithm으로 위상 정렬
pub struct JobDag {
    /// job_id → 스케줄
    jobs: HashMap<String, JobSchedule>,
    /// job_id → 이 job이 완료되어야 실행되는 job들 (dependents)
    dependents: HashMap<String, Vec<String>>,
    /// job_id → 이 job을 실행하기 위해 완료되어야 하는 job들 (prerequisites)
    prerequisites: HashMap<String, Vec<String>>,
}

impl JobDag {
    /// 새 JobDag 생성
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
            dependents: HashMap::new(),
            prerequisites: HashMap::new(),
        }
    }

    /// 작업 등록
    pub fn add_job(&mut self, id: &str, schedule: JobSchedule) {
        self.jobs.insert(id.to_string(), schedule);
        self.dependents.entry(id.to_string()).or_default();
        self.prerequisites.entry(id.to_string()).or_default();
    }

    /// 의존성 추가: `job`은 `depends_on` 완료 후 실행됨
    ///
    /// 예: `add_dependency("cleanup", "backup")` → backup 완료 후 cleanup 실행
    pub fn add_dependency(&mut self, job: &str, depends_on: &str) {
        // depends_on → job 방향으로 dependent 등록
        self.dependents
            .entry(depends_on.to_string())
            .or_default()
            .push(job.to_string());

        // job의 prerequisite에 depends_on 추가
        self.prerequisites
            .entry(job.to_string())
            .or_default()
            .push(depends_on.to_string());

        // 노드가 jobs에 없어도 그래프에 존재하도록 초기화
        self.dependents.entry(job.to_string()).or_default();
        self.prerequisites.entry(depends_on.to_string()).or_default();
    }

    /// Kahn's algorithm으로 위상 정렬된 실행 순서 반환.
    /// 순환 의존성 감지 시 `Err(DbxError::InvalidArguments)` 반환.
    pub fn resolve_execution_order(&self) -> DbxResult<Vec<String>> {
        // 모든 노드 수집 (jobs + dependency에서만 참조된 노드 포함)
        let mut all_nodes: HashSet<String> = self.jobs.keys().cloned().collect();
        for (k, vs) in &self.dependents {
            all_nodes.insert(k.clone());
            all_nodes.extend(vs.iter().cloned());
        }

        // in-degree 계산 (prerequisites 수)
        let mut in_degree: HashMap<String, usize> = all_nodes
            .iter()
            .map(|n| (n.clone(), 0))
            .collect();

        for (job, prereqs) in &self.prerequisites {
            *in_degree.entry(job.clone()).or_insert(0) += prereqs.len();
        }

        // in-degree 0인 노드부터 큐에 추가
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, v)| **v == 0)
            .map(|(k, _)| k.clone())
            .collect();

        // 결정적 순서를 위해 정렬
        let mut sorted: Vec<String> = queue.drain(..).collect();
        sorted.sort();
        let mut queue: VecDeque<String> = VecDeque::from(sorted);

        let mut order = Vec::new();

        while let Some(job) = queue.pop_front() {
            order.push(job.clone());

            if let Some(deps) = self.dependents.get(&job) {
                let mut next_batch: Vec<String> = Vec::new();
                for d in deps {
                    let cnt = in_degree.entry(d.clone()).or_insert(0);
                    if *cnt > 0 {
                        *cnt -= 1;
                    }
                    if *cnt == 0 {
                        next_batch.push(d.clone());
                    }
                }
                next_batch.sort();
                queue.extend(next_batch);
            }
        }

        // 순환 감지: 처리된 노드 수 != 전체 노드 수
        if order.len() != all_nodes.len() {
            return Err(DbxError::InvalidArguments(
                "순환 의존성 감지됨".to_string(),
            ));
        }

        Ok(order)
    }

    /// 등록된 작업 수
    pub fn job_count(&self) -> usize {
        self.jobs.len()
    }

    /// 특정 작업의 선행 작업 목록
    pub fn prerequisites_of(&self, job: &str) -> Vec<String> {
        self.prerequisites
            .get(job)
            .cloned()
            .unwrap_or_default()
    }
}

impl Default for JobDag {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_dag_dependency_ordering() {
        let mut dag = JobDag::new();
        dag.add_job("backup", JobSchedule::Interval(std::time::Duration::from_secs(3600)));
        dag.add_job("cleanup", JobSchedule::After("backup".to_string()));
        dag.add_dependency("cleanup", "backup");

        let order = dag.resolve_execution_order().unwrap();
        // backup은 cleanup보다 먼저 실행되어야 함
        let backup_pos = order.iter().position(|x| x == "backup").unwrap();
        let cleanup_pos = order.iter().position(|x| x == "cleanup").unwrap();
        assert!(backup_pos < cleanup_pos, "backup must run before cleanup");
    }

    #[test]
    fn test_job_dag_cycle_detection() {
        let mut dag = JobDag::new();
        dag.add_dependency("a", "b");
        dag.add_dependency("b", "a");
        assert!(dag.resolve_execution_order().is_err(), "순환 의존성 감지 실패");
    }

    #[test]
    fn test_job_dag_no_dependencies() {
        let mut dag = JobDag::new();
        dag.add_job("job1", JobSchedule::Manual);
        dag.add_job("job2", JobSchedule::Manual);
        dag.add_job("job3", JobSchedule::Manual);

        let order = dag.resolve_execution_order().unwrap();
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn test_job_dag_chain() {
        let mut dag = JobDag::new();
        dag.add_job("a", JobSchedule::Manual);
        dag.add_job("b", JobSchedule::After("a".to_string()));
        dag.add_job("c", JobSchedule::After("b".to_string()));
        dag.add_dependency("b", "a");
        dag.add_dependency("c", "b");

        let order = dag.resolve_execution_order().unwrap();
        assert_eq!(order.len(), 3);
        let a_pos = order.iter().position(|x| x == "a").unwrap();
        let b_pos = order.iter().position(|x| x == "b").unwrap();
        let c_pos = order.iter().position(|x| x == "c").unwrap();
        assert!(a_pos < b_pos);
        assert!(b_pos < c_pos);
    }

    #[test]
    fn test_job_dag_diamond() {
        // a → b, a → c, b → d, c → d (다이아몬드 패턴)
        let mut dag = JobDag::new();
        dag.add_job("a", JobSchedule::Manual);
        dag.add_job("b", JobSchedule::After("a".to_string()));
        dag.add_job("c", JobSchedule::After("a".to_string()));
        dag.add_job("d", JobSchedule::Manual);
        dag.add_dependency("b", "a");
        dag.add_dependency("c", "a");
        dag.add_dependency("d", "b");
        dag.add_dependency("d", "c");

        let order = dag.resolve_execution_order().unwrap();
        assert_eq!(order.len(), 4);
        let a_pos = order.iter().position(|x| x == "a").unwrap();
        let b_pos = order.iter().position(|x| x == "b").unwrap();
        let c_pos = order.iter().position(|x| x == "c").unwrap();
        let d_pos = order.iter().position(|x| x == "d").unwrap();
        assert!(a_pos < b_pos);
        assert!(a_pos < c_pos);
        assert!(b_pos < d_pos);
        assert!(c_pos < d_pos);
    }
}
