//! Deploy prism implementation for the Ultraviolet system.
//!
//! This prism provides deployment capabilities for UV to various platforms,
//! starting with AWS Lambda support.

pub mod spectrum;

use std::env;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;
use serde_json::Value;
use tracing::{info, error, debug};

use aws_config::BehaviorVersion;
use aws_sdk_lambda::{Client as LambdaClient, types::{FunctionCode, Runtime, Architecture}};
use aws_sdk_apigatewayv2::{Client as ApiGatewayV2Client, types::{ProtocolType}};

use spectrum::{LambdaRequest, LambdaResponse, DeploymentStatus};
use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

/// Deploy prism for platform deployments.
pub struct DeployPrism {
    /// The prism's spectrum (configuration)
    spectrum: Option<UVSpectrum>,
}

impl DeployPrism {
    /// Create a new Deploy prism.
    pub fn new() -> Self {
        Self {
            spectrum: None,
        }
    }

    /// Get the Lambda deployment package path.
    fn get_lambda_package_path(&self) -> Result<PathBuf> {
        let home_dir = env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| UVError::ExecutionError(format!("Failed to get home directory: {}", e)))?;
            
        let package_path = home_dir.join(".uv/prisms/system/deploy/lambda/package.zip");
        
        if !package_path.exists() {
            return Err(UVError::ExecutionError(
                format!("Lambda package not found at {}. Run 'make build-lambda-package' first.", package_path.display())
            ));
        }
        
        Ok(package_path)
    }

    /// Read the Lambda deployment package.
    fn read_lambda_package(&self) -> Result<Vec<u8>> {
        let package_path = self.get_lambda_package_path()?;
        
        debug!("Reading Lambda package from: {}", package_path.display());
        
        fs::read(&package_path)
            .map_err(|e| UVError::ExecutionError(format!("Failed to read Lambda package: {}", e)))
    }

    /// Create or update Lambda function.
    async fn create_or_update_function(
        &self,
        lambda_client: &LambdaClient,
        request: &LambdaRequest,
        package_bytes: Vec<u8>,
    ) -> Result<String> {
        debug!("Creating/updating Lambda function: {}", request.function_name);

        // First, try to get the existing function
        match lambda_client.get_function()
            .function_name(&request.function_name)
            .send()
            .await
        {
            Ok(_existing_function) => {
                // Function exists, update it
                info!("Updating existing Lambda function: {}", request.function_name);
                
                let update_result = lambda_client.update_function_code()
                    .function_name(&request.function_name)
                    .zip_file(package_bytes.into())
                    .send()
                    .await
                    .map_err(|e| UVError::ExecutionError(format!("Failed to update Lambda function: {}", e)))?;

                // Update function configuration
                lambda_client.update_function_configuration()
                    .function_name(&request.function_name)
                    .memory_size(request.memory_size)
                    .timeout(request.timeout)
                    .send()
                    .await
                    .map_err(|e| UVError::ExecutionError(format!("Failed to update function configuration: {}", e)))?;

                Ok(update_result.function_arn().unwrap_or_default().to_string())
            },
            Err(_) => {
                // Function doesn't exist, create it
                info!("Creating new Lambda function: {}", request.function_name);

                // For now, we'll use a basic execution role ARN
                // In a production implementation, we'd want to create the role if it doesn't exist
                let basic_role_arn = format!(
                    "arn:aws:iam::{}:role/lambda-basic-execution-role",
                    "123456789012" // This should be dynamically determined
                );

                let create_result = lambda_client.create_function()
                    .function_name(&request.function_name)
                    .runtime(Runtime::Providedal2023)
                    .architectures(Architecture::Arm64)
                    .role(&basic_role_arn)
                    .handler("bootstrap")
                    .code(
                        FunctionCode::builder()
                            .zip_file(package_bytes.into())
                            .build()
                    )
                    .memory_size(request.memory_size)
                    .timeout(request.timeout)
                    .description("UV Lambda WebSocket handler")
                    .send()
                    .await
                    .map_err(|e| UVError::ExecutionError(format!("Failed to create Lambda function: {}", e)))?;

                Ok(create_result.function_arn().unwrap_or_default().to_string())
            }
        }
    }

    /// Create WebSocket API Gateway.
    async fn create_websocket_api(
        &self,
        api_client: &ApiGatewayV2Client,
        function_arn: &str,
        request: &LambdaRequest,
    ) -> Result<(String, String)> {
        debug!("Creating WebSocket API Gateway");

        // Create the API
        let api_result = api_client.create_api()
            .name(format!("{}-websocket-api", request.function_name))
            .description("UV Lambda WebSocket API")
            .protocol_type(ProtocolType::Websocket)
            .route_selection_expression("$request.body.action")
            .send()
            .await
            .map_err(|e| UVError::ExecutionError(format!("Failed to create WebSocket API: {}", e)))?;

        let api_id = api_result.api_id().unwrap_or_default().to_string();
        
        // Create integration
        let integration_result = api_client.create_integration()
            .api_id(&api_id)
            .integration_type(aws_sdk_apigatewayv2::types::IntegrationType::AwsProxy)
            .integration_uri(format!("arn:aws:apigateway:{}:lambda:path/2015-03-31/functions/{}/invocations", request.region, function_arn))
            .send()
            .await
            .map_err(|e| UVError::ExecutionError(format!("Failed to create integration: {}", e)))?;

        let integration_id = integration_result.integration_id().unwrap_or_default();

        // Create routes
        for route_key in &["$connect", "$disconnect", "$default"] {
            api_client.create_route()
                .api_id(&api_id)
                .route_key(*route_key)
                .target(format!("integrations/{}", integration_id))
                .send()
                .await
                .map_err(|e| UVError::ExecutionError(format!("Failed to create route {}: {}", route_key, e)))?;
        }

        // Create deployment
        let deployment_result = api_client.create_deployment()
            .api_id(&api_id)
            .description("Initial deployment")
            .send()
            .await
            .map_err(|e| UVError::ExecutionError(format!("Failed to create deployment: {}", e)))?;

        // Create stage
        api_client.create_stage()
            .api_id(&api_id)
            .stage_name(&request.stage_name)
            .deployment_id(deployment_result.deployment_id().unwrap_or_default())
            .description("Production stage")
            .send()
            .await
            .map_err(|e| UVError::ExecutionError(format!("Failed to create stage: {}", e)))?;

        // Construct WebSocket URL
        let websocket_url = format!("wss://{}.execute-api.{}.amazonaws.com/{}", api_id, request.region, request.stage_name);

        Ok((api_id, websocket_url))
    }

    /// Handle 'lambda' frequency.
    async fn handle_lambda(&self, id: Uuid, input: Value, link: &UVLink) -> Result<()> {
        // Parse the request
        let request: LambdaRequest = serde_json::from_value(input)
            .map_err(|e| UVError::InvalidInput(format!("Invalid lambda deployment request: {}", e)))?;

        info!("Starting Lambda deployment for function: {}", request.function_name);

        // Emit initial status
        let status = DeploymentStatus {
            status: "starting".to_string(),
            message: format!("Starting Lambda deployment for function: {}", request.function_name),
            progress: Some(0),
        };
        link.emit_photon(id, serde_json::to_value(status)?)?;

        // Read Lambda package
        let package_bytes = match self.read_lambda_package() {
            Ok(bytes) => {
                info!("Lambda package loaded successfully ({} bytes)", bytes.len());
                bytes
            },
            Err(e) => {
                error!("Failed to read Lambda package: {}", e);
                link.emit_trap(id, Some(e))?;
                return Ok(());
            }
        };

        // Create AWS SDK clients
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(request.region.clone()))
            .load()
            .await;
        
        let lambda_client = LambdaClient::new(&config);
        let api_client = ApiGatewayV2Client::new(&config);

        // Emit progress
        let status = DeploymentStatus {
            status: "deploying_function".to_string(),
            message: "Creating/updating Lambda function...".to_string(),
            progress: Some(25),
        };
        link.emit_photon(id, serde_json::to_value(status)?)?;

        // Create/update Lambda function
        let function_arn = match self.create_or_update_function(&lambda_client, &request, package_bytes).await {
            Ok(arn) => {
                info!("Lambda function deployed successfully: {}", arn);
                arn
            },
            Err(e) => {
                error!("Failed to deploy Lambda function: {}", e);
                link.emit_trap(id, Some(e))?;
                return Ok(());
            }
        };

        // Emit progress
        let status = DeploymentStatus {
            status: "deploying_api".to_string(),
            message: "Creating WebSocket API Gateway...".to_string(),
            progress: Some(75),
        };
        link.emit_photon(id, serde_json::to_value(status)?)?;

        // Create API Gateway
        let (api_id, websocket_url) = match self.create_websocket_api(&api_client, &function_arn, &request).await {
            Ok((id, url)) => {
                info!("WebSocket API created successfully: {} -> {}", id, url);
                (id, url)
            },
            Err(e) => {
                error!("Failed to create WebSocket API: {}", e);
                link.emit_trap(id, Some(e))?;
                return Ok(());
            }
        };

        // Create success response
        let response = LambdaResponse {
            function_arn,
            api_id,
            websocket_url,
            deployment_time: chrono::Utc::now().to_rfc3339(),
            status: "success".to_string(),
        };

        info!("Lambda deployment completed successfully");
        link.emit_photon(id, serde_json::to_value(response)?)?;
        link.emit_trap(id, None)?;

        Ok(())
    }
}

impl UVPrism for DeployPrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        match pulse {
            UVPulse::Wavefront(wavefront) => {
                match wavefront.frequency.as_str() {
                    "lambda" => {
                        let link_clone = link.clone();
                        let input_clone = wavefront.input.clone();
                        
                        tokio::spawn(async move {
                            let deploy_prism = DeployPrism::new();
                            if let Err(e) = deploy_prism.handle_lambda(id, input_clone, &link_clone).await {
                                error!("Error in lambda deployment: {}", e);
                                let _ = link_clone.emit_trap(id, Some(e));
                            }
                        });
                        return Ok(true);
                    },
                    _ => {
                        // Unknown frequency
                        let error = UVError::MethodNotFound(wavefront.frequency.clone());
                        link.emit_trap(id, Some(error))?;
                        return Ok(true);
                    }
                }
            },
            _ => {
                // Ignore other pulse types
                return Ok(false);
            }
        }
    }
}

#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(DeployPrism::new())
}
