//! AWS Ada credentials management prism implementation.
//!
//! This prism provides capabilities for managing AWS credentials using the 
//! Ada tool and provisioning AWS accounts for credential acquisition.

pub mod spectrum;

use serde_json::json;
use spectrum::DEFAULT_REGION;
use std::fs::{self, File};
use std::io::Read;
use std::process::Command;
use uuid::Uuid;

use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

use crate::spectrum::{
    AdminInput, AdminOutput, CredentialsInput, CredentialsOutput,
    ProvisionInput, ProvisionOutput, DeployInput, DeployOutput };

/// Ada prism for AWS credentials management
pub struct AdaPrism {
    spectrum: Option<UVSpectrum>
}

impl AdaPrism {
    /// Create a new Ada prism
    pub fn new() -> Self {
        Self {
            spectrum: None
        }
    }

    /// Handle credentials frequency
    fn handle_credentials(&self, id: Uuid, input: CredentialsInput, link: &UVLink) -> Result<()> {
        // Execute ada command directly
        let output = Command::new("ada")
            .arg("credentials")
            .arg("update")
            .arg("--account")
            .arg(&input.account)
            .arg("--provider")
            .arg(&input.provider)
            .arg("--role")
            .arg(&input.role)
            .arg("--once")
            .output()
            .map_err(|e| UVError::ExecutionError(format!("Failed to execute ada: {}", e)))?;

        // Check if command was successful
        let success = output.status.success();
        let error_str = String::from_utf8_lossy(&output.stderr).to_string();

        // Prepare response
        let message = if success {
            format!("Successfully updated credentials for account {}", input.account)
        } else {
            format!("Failed to update credentials: {}", error_str)
        };

        // Emit response
        let response = CredentialsOutput {
            success,
            message,
        };

        link.emit_photon(id, serde_json::to_value(response)?)?;
        link.emit_trap(id, None)?;
        
        Ok(())
    }

    /// Handle provision frequency
    fn handle_provision(&self, id: Uuid, input: ProvisionInput, link: &UVLink) -> Result<()> {
        // First get credentials by calling ada directly
        let ada_output = Command::new("ada")
            .arg("credentials")
            .arg("update")
            .arg("--account")
            .arg(&input.account)
            .arg("--provider")
            .arg("conduit")
            .arg("--role")
            .arg("IibsAdminAccess-DO-NOT-DELETE")
            .arg("--once")
            .output()
            .map_err(|e| UVError::ExecutionError(format!("Failed to execute ada: {}", e)))?;
        
        if !ada_output.status.success() {
            let error_str = String::from_utf8_lossy(&ada_output.stderr).to_string();
            let output = ProvisionOutput {
                success: false,
                message: format!("Failed to get credentials: {}", error_str),
            };
            
            link.emit_photon(id, serde_json::to_value(output)?)?;
            link.emit_trap(id, None)?;
            
            return Ok(());
        }
        
        // Create tokio runtime
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                let output = ProvisionOutput {
                    success: false,
                    message: format!("Failed to create Tokio runtime: {}", e),
                };
                
                link.emit_photon(id, serde_json::to_value(output)?)?;
                link.emit_trap(id, None)?;
                
                return Ok(());
            }
        };
        
        // Run the async AWS operations in the runtime
        let result: std::result::Result<ProvisionOutput, String> = runtime.block_on(async {
            // Load AWS config from environment/credentials
            let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_config::Region::new(DEFAULT_REGION))
                .load()
                .await;
                
            let iam_client = aws_sdk_iam::Client::new(&config);
            
            // Define admin policy
            let admin_policy = json!({
                "Version": "2012-10-17",
                "Statement": [{
                    "Effect": "Allow",
                    "Action": "*",
                    "Resource": "*"
                }]
            });
            
            // Define trust relationships
            let assume_role_policy = json!({
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Sid": "",
                        "Effect": "Allow",
                        "Principal": {
                            "AWS": [
                                "arn:aws:iam::187792406069:user/blue-cli"
                            ]
                        },
                        "Action": "sts:AssumeRole",
                        "Condition": {
                            "StringEquals": {
                                "sts:ExternalId": "GetSecurityTokenAdmin"
                            }
                        }
                    },
                    {
                        "Sid": "",
                        "Effect": "Allow",
                        "Principal": {
                            "AWS": [
                                "arn:aws:iam::187792406069:user/blue-cli"
                            ]
                        },
                        "Action": [
                            "sts:SetSourceIdentity",
                            "sts:TagSession"
                        ]
                    },
                    {
                        "Sid": "AllowECSToAssumeRole",
                        "Effect": "Allow",
                        "Principal": {
                            "Service": "ecs-tasks.amazonaws.com"
                        },
                        "Action": "sts:AssumeRole"
                    }
                ]
            });
            
            // Create the IAM role
            let create_result = iam_client.create_role()
                .role_name("blueAdminAccess-DO-NOT-DELETE")
                .assume_role_policy_document(assume_role_policy.to_string())
                .max_session_duration(43200) // 12 hours in seconds
                .send()
                .await;
                
            if let Err(e) = create_result {
                // Extract detailed error
                let error_details = match &e {
                    aws_sdk_iam::error::SdkError::ServiceError(context) => {
                        format!(
                            "Service error: {:?}", 
                            context.err().to_string()
                        )
                    },
                    _ => format!("Non-service error: {:?}", e)
                };
                
                return Err(format!("AWS IAM error: {}", error_details));
            }
                
            // Attach admin permissions
            let policy_result = iam_client.put_role_policy()
                .role_name("blueAdminAccess-DO-NOT-DELETE")
                .policy_name("AdminAccess")
                .policy_document(admin_policy.to_string())
                .send()
                .await;
                
            if let Err(e) = policy_result {
                return Err(format!("Failed to create policy update: {}", e));
            }
                
            Ok(ProvisionOutput {
                success: true,
                message: "Successfully created IAM role 'blueAdminAccess-DO-NOT-DELETE'".to_string(),
            })
        });
        
        // Handle result
        match result {
            Ok(output) => {
                link.emit_photon(id, serde_json::to_value(output)?)?;
            },
            Err(error) => {
                let output = ProvisionOutput {
                    success: false,
                    message: error,
                };
                link.emit_photon(id, serde_json::to_value(output)?)?;
            }
        }
        
        // Signal completion
        link.emit_trap(id, None)?;
        
        Ok(())
    }

    /// Handle admin frequency
    fn handle_admin(&self, id: Uuid, input: AdminInput, link: &UVLink) -> Result<()> {
        // Create tokio runtime
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                let output = AdminOutput {
                    success: false,
                    message: format!("Failed to create Tokio runtime: {}", e),
                };
                
                link.emit_photon(id, serde_json::to_value(output)?)?;
                link.emit_trap(id, None)?;
                
                return Ok(());
            }
        };
        
        // Execute the AWS operations
        let result: std::result::Result<AdminOutput, String> = runtime.block_on(async {
            // Load broker-admin profile
            let broker_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_config::Region::new(DEFAULT_REGION))
                .load()
                .await;
                
            let admin_sts_client = aws_sdk_sts::Client::new(&broker_config);
            
            // Assume the role
            let admin_role_result = admin_sts_client
                .assume_role()
                .role_arn(&format!("arn:aws:iam::{}:role/blueAdminAccess-DO-NOT-DELETE", input.account))
                .role_session_name("BlueAdminSession")
                .duration_seconds(43200)  // 12 hours for max session duration
                .external_id("GetSecurityTokenAdmin")
                .send()
                .await;
                
            // Check for errors
            let admin_credentials = match admin_role_result {
                Ok(result) => {
                    match result.credentials() {
                        Some(creds) => creds.clone(),
                        None => return Err("No credentials returned from admin role assumption".to_string()),
                    }
                },
                Err(e) => {
                    return Err(format!("Failed to assume blueAdminAccess role: {:?}", e));
                }
            };
                
            // Write credentials to ~/.aws/credentials
            let home_dir = match dirs::home_dir() {
                Some(dir) => dir,
                None => return Err("Could not determine home directory".to_string()),
            };
            
            let credentials_path = home_dir.join(".aws").join("credentials");
            
            // Read existing credentials file
            let mut credential_content = String::new();
            if credentials_path.exists() {
                match File::open(&credentials_path) {
                    Ok(mut file) => {
                        if let Err(e) = file.read_to_string(&mut credential_content) {
                            return Err(format!("Failed to read credentials file: {}", e));
                        }
                    },
                    Err(e) => return Err(format!("Failed to open credentials file: {}", e)),
                }
            }
            
            // Format new default profile
            let default_profile_content = format!(
                "[default]\naws_access_key_id = {}\naws_secret_access_key = {}\naws_session_token = {}\n",
                admin_credentials.access_key_id(),
                admin_credentials.secret_access_key(),
                admin_credentials.session_token()
            );
            
            // Parse and update the credentials file
            let mut lines = credential_content.lines().collect::<Vec<_>>();
            let mut in_default_section = false;
            let mut default_section_start = 0;
            let mut default_section_end = 0;

            // Find default section boundaries
            for (i, line) in lines.iter().enumerate() {
                let line_trimmed = line.trim();
                if line_trimmed == "[default]" {
                    in_default_section = true;
                    default_section_start = i;
                } else if in_default_section && line_trimmed.starts_with('[') {
                    default_section_end = i;
                    break;
                }
            }

            // If default section was found, remove it
            if in_default_section {
                lines.drain(default_section_start..default_section_end);
                // Insert new default profile at the same position
                for (_i, line) in default_profile_content.lines().rev().enumerate() {
                    lines.insert(default_section_start, line);
                }
            } else {
                // No default section found, append it
                for line in default_profile_content.lines() {
                    lines.push(line);
                }
            }
            
            // Ensure .aws directory exists
            if let Err(e) = fs::create_dir_all(home_dir.join(".aws")) {
                return Err(format!("Failed to create .aws directory: {}", e));
            }
            
            // Write updated content back
            if let Err(e) = fs::write(&credentials_path, lines.join("\n")) {
                return Err(format!("Failed to write credentials file: {}", e));
            }
            
            Ok(AdminOutput {
                success: true,
                message: "Successfully obtained temporary credentials for blueAdminAccess-DO-NOT-DELETE role".to_string(),
            })
        });
        
        // Handle result
        match result {
            Ok(output) => {
                link.emit_photon(id, serde_json::to_value(output)?)?;
            },
            Err(error) => {
                let output = AdminOutput {
                    success: false,
                    message: error,
                };
                link.emit_photon(id, serde_json::to_value(output)?)?;
            }
        }
        
        // Signal completion
        link.emit_trap(id, None)?;
        
        Ok(())
    }
    
    /// Handle deploy frequency to deploy UV server to ECS Fargate
    fn handle_deploy(&self, id: Uuid, input: DeployInput, link: &UVLink) -> Result<()> {
        // Create tokio runtime
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                let output = DeployOutput {
                    success: false,
                    message: format!("Failed to create Tokio runtime: {}", e),
                    task_arn: None,
                    public_ip: None,
                };
                
                link.emit_photon(id, serde_json::to_value(output)?)?;
                link.emit_trap(id, None)?;
                
                return Ok(());
            }
        };
        
        // Execute the AWS operations
        let result = runtime.block_on(async {
            // First, assume the blue admin role to get credentials
            let broker_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_config::Region::new(DEFAULT_REGION))
                .load()
                .await;
            
            let ecs_client = aws_sdk_ecs::Client::new(&broker_config);
            
            // Create or update ECS task definition
            let task_def_resp = ecs_client.register_task_definition()
                .family("ultraviolet-server")
                .cpu(input.cpu.clone())
                .memory(input.memory.clone())
                .network_mode(aws_sdk_ecs::types::NetworkMode::Awsvpc)
                .requires_compatibilities(aws_sdk_ecs::types::Compatibility::Fargate)
                .execution_role_arn(format!("arn:aws:iam::{}:role/blueAdminAccess-DO-NOT-DELETE", input.account))
                .task_role_arn(format!("arn:aws:iam::{}:role/blueAdminAccess-DO-NOT-DELETE", input.account))
                .container_definitions(
                    aws_sdk_ecs::types::ContainerDefinition::builder()
                        .name("ultraviolet-server")
                        .image(input.docker_image.clone())
                        .essential(true)
                        .port_mappings(
                            aws_sdk_ecs::types::PortMapping::builder()
                                .container_port(3000)
                                .host_port(3000)
                                .protocol(aws_sdk_ecs::types::TransportProtocol::Tcp)
                                .build()
                        )
                        .environment(
                            aws_sdk_ecs::types::KeyValuePair::builder()
                                .name("HOME")
                                .value("/home/uvuser")
                                .build()
                        )
                        .environment(
                            aws_sdk_ecs::types::KeyValuePair::builder()
                                .name("TS_AUTHKEY")
                                .value(input.tailscale_authkey.clone())
                                .build()
                        )
                        .log_configuration(
                            aws_sdk_ecs::types::LogConfiguration::builder()
                                .log_driver(aws_sdk_ecs::types::LogDriver::Awslogs)
                                // These should be specified as key-value pairs properly
                                .options("awslogs-group", "ultraviolet-logs")
                                .options("awslogs-region", &input.region)
                                .options("awslogs-stream-prefix", "ultraviolet")
                                .options("awslogs-create-group", "true")
                                .build()
                                .ok()
                                .expect("")
                        )
                        .build()
                )
                .send()
                .await;
            
            let task_def_arn = match task_def_resp {
                Ok(resp) => resp.task_definition()
                    .and_then(|td| td.task_definition_arn())
                    .map(|arn| arn.to_string())
                    .ok_or_else(|| "No task definition ARN in response".to_string())?,
                Err(e) => return Err(format!("Failed to register task definition: {}", e.into_service_error())),
            };
            
            // Create default cluster if it doesn't exist
            let _ = ecs_client.create_cluster()
                .cluster_name("ultraviolet-cluster")
                .send()
                .await;

            // Fetch the Default VPC and subnet
            let ec2_client = aws_sdk_ec2::Client::new(&broker_config);

            // Get default VPC and subnets for task placement
            let vpcs_result = ec2_client.describe_vpcs()
            .filters(aws_sdk_ec2::types::Filter::builder()
                .name("isDefault")
                .values("true")
                .build())
            .send()
            .await;

            let default_vpc_id = match vpcs_result {
            Ok(resp) => {
                match resp.vpcs().first() {
                    Some(vpc) => vpc.vpc_id().map(|id| id.to_string()),
                    None => return Err("No default VPC found in the account".to_string()),
                }
            },
            Err(e) => return Err(format!("Failed to describe VPCs: {:?}", e)),
            };

            // Get public subnets in the default VPC
            let vpc_id = match default_vpc_id {
                Some(id) => id,
                None => return Err("Default VPC ID is missing".to_string()),
            };

            let subnets_result = ec2_client.describe_subnets()
            .filters(aws_sdk_ec2::types::Filter::builder()
                .name("vpc-id")
                .values(vpc_id.clone())
                .build())
            .send()
            .await;

            let subnet_id = subnets_result
                .map_err(|_| "Unable to find subnet".to_string())?
                .subnets()
                .first()
                .map(|subnet| subnet.subnet_id())
                .map(|id| id.expect("").to_string())
                .ok_or_else(|| "No subnet ID found in default VPC".to_string())?;
                        
            // Run the task in Fargate
            let run_task_resp = ecs_client.run_task()
                .task_definition(task_def_arn.clone())
                .cluster("ultraviolet-cluster")
                .count(1)
                .launch_type(aws_sdk_ecs::types::LaunchType::Fargate)
                .network_configuration(
                    aws_sdk_ecs::types::NetworkConfiguration::builder()
                        .awsvpc_configuration(
                            aws_sdk_ecs::types::AwsVpcConfiguration::builder()
                                .subnets(subnet_id)  // This would need to be fetched dynamically
                                .assign_public_ip(aws_sdk_ecs::types::AssignPublicIp::Enabled)
                                .build()
                                .expect("")
                        )
                        .build()
                )
                .send()
                .await;
            
            let task_arn = match run_task_resp {
                Ok(resp) => resp.tasks()
                    .first()
                    .expect("")
                    .task_arn()
                    .map(|v| v.to_string()),
                Err(e) => return Err(format!("Failed to run task: {}", e.into_service_error())),
            };
            
            Ok(DeployOutput {
                success: true,
                message: "Successfully deployed Ultraviolet server to ECS".to_string(),
                task_arn,
                public_ip: Some("Task starting, public IP will be available soon".to_string()),
            })
        });
        
        // Handle the result
        match result {
            Ok(output) => {
                link.emit_photon(id, serde_json::to_value(output)?)?;
            },
            Err(e) => {
                println!("{e}");
                let output = DeployOutput {
                    success: false,
                    message: e,
                    task_arn: None,
                    public_ip: None,
                };
                link.emit_photon(id, serde_json::to_value(output)?)?;
            },
        }
        
        // Signal completion
        link.emit_trap(id, None)?;
        
        Ok(())
    }
}

impl UVPrism for AdaPrism {
    fn init(&mut self, spectrum: UVSpectrum) -> Result<()> {
        self.spectrum = Some(spectrum);
        Ok(())
    }
    
    fn handle_pulse(&self, id: Uuid, pulse: &UVPulse, link: &UVLink) -> Result<bool> {
        if let UVPulse::Wavefront(wavefront) = pulse {
            match wavefront.frequency.as_str() {
                "credentials" => {
                    let input: CredentialsInput = serde_json::from_value(wavefront.input.clone())?;
                    self.handle_credentials(id, input, link)?;
                    return Ok(true);
                },
                "provision" => {
                    let input: ProvisionInput = serde_json::from_value(wavefront.input.clone())?;
                    self.handle_provision(id, input, link)?;
                    return Ok(true);
                },
                "admin" => {
                    let input: AdminInput = serde_json::from_value(wavefront.input.clone())?;
                    self.handle_admin(id, input, link)?;
                    return Ok(true);
                },
                "deploy" => {
                    let input: DeployInput = serde_json::from_value(wavefront.input.clone())?;
                    match self.handle_deploy(id, input, link) {
                        Ok(_) => return Ok(true),
                        Err(e) => println!("{e}"),
                    }
                    return Ok(true);
                },
                _ => {
                    // Unknown frequency
                    let error = UVError::MethodNotFound(wavefront.frequency.clone());
                    link.emit_trap(id, Some(error))?;
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
}

// Export a function to create a new instance
// This will be used by the dynamic loading system
#[no_mangle]
pub fn create_prism() -> Box<dyn UVPrism> {
    Box::new(AdaPrism::new())
}
