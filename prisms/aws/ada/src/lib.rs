//! AWS Ada credentials management prism implementation.
//!
//! This prism provides capabilities for managing AWS credentials using the 
//! Ada tool and provisioning AWS accounts for credential acquisition.

pub mod spectrum;

use serde_json::json;
use std::fs::{self, File};
use std::io::Read;
use std::process::Command;
use uuid::Uuid;

use uv_core::{
    Result, UVError, UVLink, UVPrism, UVPulse, UVSpectrum
};

use crate::spectrum::{
    AdminInput, AdminOutput, CredentialsInput, CredentialsOutput,
    ProvisionInput, ProvisionOutput };

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
            let config = aws_config::from_env()
                .region("us-east-1")
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
                                "arn:aws:iam::621463273587:role/BrokerService-Prod",
                                "arn:aws:iam::048851363556:role/BrokerService-Prod",
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
                                "arn:aws:iam::621463273587:role/BrokerService-Prod",
                                "arn:aws:iam::048851363556:role/BrokerService-Prod",
                                "arn:aws:iam::187792406069:user/blue-cli"
                            ]
                        },
                        "Action": [
                            "sts:SetSourceIdentity",
                            "sts:TagSession"
                        ]
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
            let broker_config = aws_config::from_env()
                .profile_name("broker-admin")
                .region("us-east-1")
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
                admin_credentials.access_key_id().unwrap_or_default(),
                admin_credentials.secret_access_key().unwrap_or_default(),
                admin_credentials.session_token().unwrap_or_default()
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
