use crate::domain::entities::Project;
use crate::domain::error::DomainResult;

/// Repository trait for Project entity
/// Infrastructure layer will implement this trait
#[allow(dead_code)]
pub trait ProjectRepository: Send + Sync {
    fn insert(&self, project: &Project) -> DomainResult<()>;
    fn find_by_key(&self, key: &str) -> DomainResult<Option<Project>>;
    fn find_all(&self) -> DomainResult<Vec<Project>>;
}
