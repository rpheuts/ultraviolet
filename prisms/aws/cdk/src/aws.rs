//! AWS client utilities and shared functions.

use aws_config::ConfigLoader;
use std::fmt;
use uv_core::Result;
use uv_core::UVError;

/// Create an S3 client
pub async fn get_s3_client(region: Option<&str>) -> Result<aws_sdk_s3::Client> {
    let config = get_config(region).await?;
    Ok(aws_sdk_s3::Client::new(&config))
}

/// Create an IAM client
pub async fn get_iam_client(region: Option<&str>) -> Result<aws_sdk_iam::Client> {
    let config = get_config(region).await?;
    Ok(aws_sdk_iam::Client::new(&config))
}

/// Create an ECR client
pub async fn get_ecr_client(region: Option<&str>) -> Result<aws_sdk_ecr::Client> {
    let config = get_config(region).await?;
    Ok(aws_sdk_ecr::Client::new(&config))
}

/// Create a CloudWatch Logs client
pub async fn get_logs_client(region: Option<&str>) -> Result<aws_sdk_cloudwatchlogs::Client> {
    let config = get_config(region).await?;
    Ok(aws_sdk_cloudwatchlogs::Client::new(&config))
}

/// Create a CloudFormation client
pub async fn get_cloudformation_client(region: Option<&str>) -> Result<aws_sdk_cloudformation::Client> {
    let config = get_config(region).await?;
    Ok(aws_sdk_cloudformation::Client::new(&config))
}

/// Shared function to create AWS config with specified region
async fn get_config(region: Option<&str>) -> Result<aws_types::SdkConfig> {
    let builder = ConfigLoader::default();
    
    let builder = if let Some(region) = region {
        builder.region(aws_config::Region::new(region.to_string()))
    } else {
        builder
    };
    
    Ok(builder.load().await)
}

/// Result of a cleanup operation
#[derive(Debug, Clone)]
pub enum CleanupStatus {
    Success,
    Failure(String),
    Skipped(String),
    DryRun,
}

impl fmt::Display for CleanupStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CleanupStatus::Success => write!(f, "SUCCESS"),
            CleanupStatus::Failure(reason) => write!(f, "FAILURE: {}", reason),
            CleanupStatus::Skipped(reason) => write!(f, "SKIPPED: {}", reason),
            CleanupStatus::DryRun => write!(f, "DRY_RUN"),
        }
    }
}

/// Convert AWS SDK error to UV error
pub fn aws_err_to_uv_err<E: fmt::Display>(e: E) -> UVError {
    UVError::Other(format!("AWS Error: {}", e))
}
