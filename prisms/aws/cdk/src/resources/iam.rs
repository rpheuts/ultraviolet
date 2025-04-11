//! IAM role resource handler.

use crate::aws::{get_iam_client, CleanupStatus};
use crate::resources::{ResourceHandler, CleanupFuture};
use std::fmt::Debug;
use futures::future::FutureExt;

/// AWS IAM role resource type
pub const IAM_ROLE_RESOURCE_TYPE: &str = "AWS::IAM::Role";

/// IAM role handler
#[derive(Debug)]
pub struct IAMRoleHandler;

impl ResourceHandler for IAMRoleHandler {
    fn can_handle(&self, resource_type: &str) -> bool {
        resource_type == IAM_ROLE_RESOURCE_TYPE
    }
    
    fn delete_resource<'a>(&'a self, physical_id: &'a str, region: Option<&'a str>, dry_run: bool) -> CleanupFuture<'a> {
        iam_delete_resource(physical_id, region, dry_run)
    }
}

// Separate function to avoid issues with async+boxed future syntax
fn iam_delete_resource<'a>(physical_id: &'a str, region: Option<&'a str>, dry_run: bool) -> CleanupFuture<'a> {
    async move {
        if dry_run {
            return Ok(CleanupStatus::DryRun);
        }
        
        let client = get_iam_client(region).await?;
        
        // First, detach any policies from the role
        match client.list_attached_role_policies().role_name(physical_id).send().await {
            Ok(response) => {
                let policies = response.attached_policies();
                for policy in policies {
                    if let Some(policy_arn) = policy.policy_arn() {
                        match client.detach_role_policy()
                            .role_name(physical_id)
                            .policy_arn(policy_arn)
                            .send()
                            .await {
                            Ok(_) => {},
                            Err(e) => {
                                log::warn!("Failed to detach policy {}: {}", policy_arn, e);
                            }
                        }
                    }
                }
            },
            Err(e) => {
                log::warn!("Failed to list attached policies for role {}: {}", physical_id, e);
                // Continue with deletion even if this fails
            }
        }
        
        // Try to delete the role
        match client.delete_role().role_name(physical_id).send().await {
            Ok(_) => Ok(CleanupStatus::Success),
            Err(e) => {
                // Check if the error is NoSuchEntity
                if let Some(sdk_err) = e.as_service_error() {
                    if sdk_err.is_no_such_entity_exception() {
                        return Ok(CleanupStatus::Skipped("Role does not exist".to_string()));
                    }
                }
                Ok(CleanupStatus::Failure(format!("Failed to delete role: {}", e)))
            }
        }
    }.boxed()
}
