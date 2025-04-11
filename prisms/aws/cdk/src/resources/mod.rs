//! Resource-specific handlers for AWS resource management.

pub mod s3;
pub mod iam;
pub mod ecr;
pub mod logs;

use crate::aws::CleanupStatus;
use std::fmt::Debug;
use uv_core::Result;
use futures::future::BoxFuture;

/// Type alias for the cleanup future
pub type CleanupFuture<'a> = BoxFuture<'a, Result<CleanupStatus>>;

/// Trait for resource-specific handlers
pub trait ResourceHandler: Send + Sync + Debug {
    /// Check if this handler can handle the given resource type
    fn can_handle(&self, resource_type: &str) -> bool;
    
    /// Delete a resource with the given physical ID
    fn delete_resource<'a>(&'a self, physical_id: &'a str, region: Option<&'a str>, dry_run: bool) -> CleanupFuture<'a>;
}

/// Enum of resource handlers for dynamic dispatch
#[derive(Debug)]
pub enum ResourceHandlerEnum {
    S3(s3::S3Handler),
    IAM(iam::IAMRoleHandler),
    ECR(ecr::ECRHandler),
    Logs(logs::LogGroupHandler),
}

impl ResourceHandler for ResourceHandlerEnum {
    fn can_handle(&self, resource_type: &str) -> bool {
        match self {
            ResourceHandlerEnum::S3(h) => h.can_handle(resource_type),
            ResourceHandlerEnum::IAM(h) => h.can_handle(resource_type),
            ResourceHandlerEnum::ECR(h) => h.can_handle(resource_type),
            ResourceHandlerEnum::Logs(h) => h.can_handle(resource_type),
        }
    }

    fn delete_resource<'a>(&'a self, physical_id: &'a str, region: Option<&'a str>, dry_run: bool) -> CleanupFuture<'a> {
        match self {
            ResourceHandlerEnum::S3(h) => h.delete_resource(physical_id, region, dry_run),
            ResourceHandlerEnum::IAM(h) => h.delete_resource(physical_id, region, dry_run),
            ResourceHandlerEnum::ECR(h) => h.delete_resource(physical_id, region, dry_run),
            ResourceHandlerEnum::Logs(h) => h.delete_resource(physical_id, region, dry_run),
        }
    }
}

/// Get a handler for the specified resource type
pub fn get_handler(resource_type: &str) -> Option<ResourceHandlerEnum> {
    match resource_type {
        s3::S3_RESOURCE_TYPE => Some(ResourceHandlerEnum::S3(s3::S3Handler)),
        iam::IAM_ROLE_RESOURCE_TYPE => Some(ResourceHandlerEnum::IAM(iam::IAMRoleHandler)),
        ecr::ECR_REPO_RESOURCE_TYPE => Some(ResourceHandlerEnum::ECR(ecr::ECRHandler)),
        logs::LOG_GROUP_RESOURCE_TYPE => Some(ResourceHandlerEnum::Logs(logs::LogGroupHandler)),
        _ => None,
    }
}

/// Common resource types that can be cleaned up
pub const COMMON_RESOURCE_TYPES: &[&str] = &[
    s3::S3_RESOURCE_TYPE,
    iam::IAM_ROLE_RESOURCE_TYPE,
    ecr::ECR_REPO_RESOURCE_TYPE,
    logs::LOG_GROUP_RESOURCE_TYPE,
];

/// Status filters for resource cleanup
pub enum StatusFilter {
    DeleteSkipped,
    DeleteFailed,
    All,
}

impl StatusFilter {
    /// Parse status filter from string
    pub fn from_str(s: &str) -> Self {
        match s {
            "DELETE_FAILED" => StatusFilter::DeleteFailed,
            "ALL" => StatusFilter::All,
            _ => StatusFilter::DeleteSkipped,
        }
    }
    
    /// Check if a resource status matches the filter
    pub fn matches(&self, status: Option<&str>) -> bool {
        match (self, status) {
            (StatusFilter::DeleteSkipped, Some("DELETE_SKIPPED")) => true,
            (StatusFilter::DeleteFailed, Some("DELETE_FAILED")) => true,
            (StatusFilter::All, _) => true,
            _ => false,
        }
    }
}
