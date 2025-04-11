//! CloudWatch Logs resource handler.

use crate::aws::{get_logs_client, CleanupStatus};
use crate::resources::{ResourceHandler, CleanupFuture};
use std::fmt::Debug;
use futures::future::FutureExt;

/// AWS CloudWatch Logs group resource type
pub const LOG_GROUP_RESOURCE_TYPE: &str = "AWS::Logs::LogGroup";

/// CloudWatch Logs group handler
#[derive(Debug)]
pub struct LogGroupHandler;

impl ResourceHandler for LogGroupHandler {
    fn can_handle(&self, resource_type: &str) -> bool {
        resource_type == LOG_GROUP_RESOURCE_TYPE
    }
    
    fn delete_resource<'a>(&'a self, physical_id: &'a str, region: Option<&'a str>, dry_run: bool) -> CleanupFuture<'a> {
        logs_delete_resource(physical_id, region, dry_run)
    }
}

// Separate function to avoid issues with async+boxed future syntax
fn logs_delete_resource<'a>(physical_id: &'a str, region: Option<&'a str>, dry_run: bool) -> CleanupFuture<'a> {
    async move {
        if dry_run {
            return Ok(CleanupStatus::DryRun);
        }
        
        let client = get_logs_client(region).await?;
        
        // Delete the log group
        match client.delete_log_group()
            .log_group_name(physical_id)
            .send()
            .await {
            Ok(_) => Ok(CleanupStatus::Success),
            Err(e) => {
                // Check if the error is ResourceNotFoundException
                if let Some(sdk_err) = e.as_service_error() {
                    if sdk_err.is_resource_not_found_exception() {
                        return Ok(CleanupStatus::Skipped("Log group does not exist".to_string()));
                    }
                }
                Ok(CleanupStatus::Failure(format!("Failed to delete log group: {}", e)))
            }
        }
    }.boxed()
}
