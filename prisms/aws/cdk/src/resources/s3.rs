//! S3 bucket resource handler.

use crate::aws::{get_s3_client, CleanupStatus};
use crate::resources::{ResourceHandler, CleanupFuture};
use std::fmt::Debug;
use futures::future::FutureExt;

/// AWS S3 bucket resource type
pub const S3_RESOURCE_TYPE: &str = "AWS::S3::Bucket";

/// S3 bucket handler
#[derive(Debug)]
pub struct S3Handler;

impl ResourceHandler for S3Handler {
    fn can_handle(&self, resource_type: &str) -> bool {
        resource_type == S3_RESOURCE_TYPE
    }
    
    fn delete_resource<'a>(&'a self, physical_id: &'a str, region: Option<&'a str>, dry_run: bool) -> CleanupFuture<'a> {
        s3_delete_resource(physical_id, region, dry_run)
    }
}

// Separate function to avoid issues with async+boxed future syntax
fn s3_delete_resource<'a>(physical_id: &'a str, region: Option<&'a str>, dry_run: bool) -> CleanupFuture<'a> {
    async move {
        if dry_run {
            return Ok(CleanupStatus::DryRun);
        }
        
        let client = get_s3_client(region).await?;
        
        // First, try to list objects to check if the bucket exists and has contents
        match client.list_objects_v2().bucket(physical_id).send().await {
            Ok(list_result) => {
                // Check if bucket has contents that need to be deleted first
                let contents = list_result.contents();
                if !contents.is_empty() {
                    // Delete all objects first
                    let mut delete_errors = false;
                    for object in contents {
                        if let Some(key) = object.key() {
                            if let Err(e) = client.delete_object()
                                .bucket(physical_id)
                                .key(key)
                                .send()
                                .await {
                                log::warn!("Failed to delete object {}: {}", key, e);
                                delete_errors = true;
                            }
                        }
                    }
                    
                    if delete_errors {
                        return Ok(CleanupStatus::Failure(
                            "Could not delete all objects in the bucket".to_string()
                        ));
                    }
                }
                
                // Now delete the bucket
                match client.delete_bucket().bucket(physical_id).send().await {
                    Ok(_) => Ok(CleanupStatus::Success),
                    Err(e) => Ok(CleanupStatus::Failure(format!("Failed to delete bucket: {}", e))),
                }
            },
            Err(e) => {
                // Check if the error is NoSuchBucket
                if let Some(sdk_err) = e.as_service_error() {
                    if sdk_err.is_no_such_bucket() {
                        return Ok(CleanupStatus::Skipped("Bucket does not exist".to_string()));
                    }
                }
                Ok(CleanupStatus::Failure(format!("Error checking bucket: {}", e)))
            }
        }
    }.boxed()
}
