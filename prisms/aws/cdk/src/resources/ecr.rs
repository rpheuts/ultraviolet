//! ECR repository resource handler.

use crate::aws::{get_ecr_client, CleanupStatus};
use crate::resources::{ResourceHandler, CleanupFuture};
use std::fmt::Debug;
use futures::future::FutureExt;

/// AWS ECR repository resource type
pub const ECR_REPO_RESOURCE_TYPE: &str = "AWS::ECR::Repository";

/// ECR repository handler
#[derive(Debug)]
pub struct ECRHandler;

impl ResourceHandler for ECRHandler {
    fn can_handle(&self, resource_type: &str) -> bool {
        resource_type == ECR_REPO_RESOURCE_TYPE
    }
    
    fn delete_resource<'a>(&'a self, physical_id: &'a str, region: Option<&'a str>, dry_run: bool) -> CleanupFuture<'a> {
        ecr_delete_resource(physical_id, region, dry_run)
    }
}

// Separate function to avoid issues with async+boxed future syntax
fn ecr_delete_resource<'a>(physical_id: &'a str, region: Option<&'a str>, dry_run: bool) -> CleanupFuture<'a> {
    async move {
        if dry_run {
            return Ok(CleanupStatus::DryRun);
        }
        
        let client = get_ecr_client(region).await?;
        
        // Delete the repository and all images within it
        match client.delete_repository()
            .repository_name(physical_id)
            .force(true) // Force delete all images
            .send()
            .await {
            Ok(_) => Ok(CleanupStatus::Success),
            Err(e) => {
                // Check if the error is RepositoryNotFoundException
                if let Some(sdk_err) = e.as_service_error() {
                    if sdk_err.is_repository_not_found_exception() {
                        return Ok(CleanupStatus::Skipped("Repository does not exist".to_string()));
                    }
                }
                Ok(CleanupStatus::Failure(format!("Failed to delete repository: {}", e)))
            }
        }
    }.boxed()
}
