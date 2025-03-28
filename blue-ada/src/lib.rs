use blue_core::prelude::*;
use serde_json::json;
use std::path::PathBuf;
use std::io::Read;

pub struct AdaModule {
    manifest: ModuleManifest,
    context: ModuleContext,
}

impl AdaModule {
    pub fn new() -> Result<Self> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| Error::Module(format!("Failed to get home directory: {}", e)))?;

        let lib_path = home.join(".blue/modules");
        let manifest_path = lib_path.join("aws").join("ada").join("manifest.toml");
        let manifest = ModuleManifest::load(&manifest_path)?;
        let context = ModuleContext::new(lib_path);

        Ok(Self {
            manifest,
            context,
        })
    }

    fn handle_ada(&mut self, args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> Result<Value> {
        // Get required account number
        let account = args.get("account")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Module("Missing required argument: account".to_string()))?;

        // Get optional provider and role with defaults
        let provider = args.get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("conduit");
        let role = args.get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("IibsAdminAccess-DO-NOT-DELETE");

        // List existing ada processes
        let processes = self.context.call_module("blue:core-process", &["list"], json!({}), None, None)?;
        
        // Find and stop any existing ada processes
        if let Some(process_list) = processes.get("processes").and_then(|v| v.as_array()) {
            for process in process_list {
                if let Some(cmd) = process.get("config")
                    .and_then(|c| c.get("command"))
                    .and_then(|c| c.as_str())
                {
                    if cmd.contains("ada") {
                        if let Some(id) = process.get("id").and_then(|i| i.as_str()) {
                            // Stop the process
                            let stop_args = json!({
                                "id": id,
                                "timeout": 5.0
                            });
                            self.context.call_module("blue:core-process", &["stop"], stop_args, None, None)?;

                            // Remove the process
                            let remove_args = json!({
                                "id": id
                            });
                            self.context.call_module("blue:core-process", &["remove"], remove_args, None, None)?;
                        }
                    }
                }
            }
        }

        // Start new ada process
        let start_args = json!({
            "command": "ada",
            "args": [
                "credentials",
                "update",
                "--account",
                account,
                "--provider",
                provider,
                "--role",
                role
            ]
        });

        let _result = self.context.call_module(
            "blue:core-process",
            &["start"],
            start_args,
            stdout,
            stderr
        )?;

        Ok(json!({
            "success": true,
            "message": format!("Started ada process for account {}", account)
        }))
    }

    fn handle_status(&mut self, _args: Value, _stdout: Option<&mut dyn Write>, _stderr: Option<&mut dyn Write>) -> Result<Value> {
        // List all processes
        let processes = self.context.call_module("blue:core-process", &["list"], json!({}), None, None)?;
        
        let mut ada_processes = Vec::new();
        
        if let Some(process_list) = processes.get("processes").and_then(|v| v.as_array()) {
            for process in process_list {
                if let Some(cmd) = process.get("config")
                    .and_then(|c| c.get("command"))
                    .and_then(|c| c.as_str())
                {
                    if cmd.contains("ada ") {
                        // Extract account number from args array
                        let account = process.get("config")
                            .and_then(|c| c.get("command"))
                            .and_then(|c| c.as_str())
                            .and_then(|cmd| {
                                cmd.split_whitespace()
                                    .skip_while(|&arg| arg != "--account")
                                    .nth(1)
                            })
                            .unwrap_or("unknown");

                        ada_processes.push(json!({
                            "account": account,
                            "started_at": process.get("started_at"),
                            "pid": process.get("pid")
                        }));
                    }
                }
            }
        }

        Ok(json!({
            "processes": ada_processes
        }))
    }

    fn handle_provision(&mut self, args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> std::result::Result<Value, Error> {
        self.handle_ada(args, stdout, stderr)?;
        
        // Create a new Tokio runtime for this operation
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Module(format!("Failed to create Tokio runtime: {}", e)))?;
        
        // Execute the async AWS SDK operations by blocking on the runtime
        runtime.block_on(async {
            // Load AWS config from the environment/credentials file
            let config = aws_config::from_env()
                .region("us-east-1")
                .load()
                .await;
            let iam_client = aws_sdk_iam::Client::new(&config);
            
            // Define admin policy as inline policy
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
            iam_client.create_role()
                .role_name("blueAdminAccess-DO-NOT-DELETE")
                .assume_role_policy_document(assume_role_policy.to_string())
                .max_session_duration(43200) // 12 hours in seconds
                .send()
                .await
                .map_err(|e| {
                    // Extract the detailed SdkError information
                    let error_details = match &e {
                        aws_sdk_iam::error::SdkError::ServiceError(context) => {
                            // This gives us the actual AWS error with code and message
                            format!(
                                "Service error: {:?}, Message: {:?}", 
                                context.err().to_string(), 
                                ""
                            )
                        },
                        _ => format!("Non-service error: {:?}", e)
                    };
                    
                    println!("AWS Error: {}", error_details);
                    
                    
                    Error::Module(format!("AWS IAM error: {}", error_details))
                })?;
                
            // Attach admin permissions
            iam_client.put_role_policy()
                .role_name("blueAdminAccess-DO-NOT-DELETE")
                .policy_name("AdminAccess")
                .policy_document(admin_policy.to_string())
                .send()
                .await
                .map_err(|e| Error::Module(format!("Failed to create policy update: {}", e)))?;
                
            std::result::Result::<_, Error>::Ok(()) // Return Result type that matches outer function
        })?;
        
        // Return success JSON response
        Ok(json!({
            "success": true,
            "message": "Successfully created IAM role 'blueAdminAccess-DO-NOT-DELETE'"
        }))
    }

    fn handle_creds(&mut self, args: Value, mut stdout: Option<&mut dyn Write>, mut stderr: Option<&mut dyn Write>) -> Result<Value> {
        // Get optional account number - default to 187792406069 if not provided
        let account = args.get("account")
            .and_then(|v| v.as_str())
            .unwrap_or("187792406069");

        // Create a new Tokio runtime for this operation
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| Error::Module(format!("Failed to create Tokio runtime: {}", e)))?;
        
        // Execute the async AWS SDK operations by blocking on the runtime
        runtime.block_on(async {
            if let Some(stdout) = stdout.as_mut() {
                writeln!(stdout, "Starting credential chain for secure access...")?;
            }
            
            // Step 1: Load the broker-admin profile credentials
            if let Some(stdout) = stdout.as_mut() {
                writeln!(stdout, "Loading broker-admin profile credentials...")?;
            }
            
            let broker_config = aws_config::from_env()
                .profile_name("broker-admin")  // Use broker-admin profile
                .region("us-east-1")
                .load()
                .await;
            
            // Step 2: Use the broker credentials to assume the BrokerService-Prod role
            if let Some(stdout) = stdout.as_mut() {
                writeln!(stdout, "Assuming BrokerService-Prod role...")?;
            }
            
            // Step 4: Use the broker role to assume the blueAdminAccess-DO-NOT-DELETE role
            if let Some(stdout) = stdout.as_mut() {
                writeln!(stdout, "Assuming blueAdminAccess-DO-NOT-DELETE role...")?;
            }
            
            let admin_sts_client = aws_sdk_sts::Client::new(&broker_config);
            
            let admin_role_result = admin_sts_client
                .assume_role()
                .role_arn(&format!("arn:aws:iam::{}:role/blueAdminAccess-DO-NOT-DELETE", account))
                .role_session_name("BlueAdminSession")
                .duration_seconds(43200)  // 12 hours for max session duration
                .external_id("GetSecurityTokenAdmin")
                .send()
                .await
                .map_err(|e| {
                    if let Some(stderr) = stderr.as_mut() {
                        let _ = writeln!(stderr, "Failed to assume blueAdminAccess role: {:?}", e);
                    }
                    Error::Module(format!("Failed to assume blueAdminAccess role: {:?}", e))
                })?;
                
            // Extract final credentials
            let admin_credentials = admin_role_result
                .credentials()
                .ok_or_else(|| Error::Module("No credentials returned from admin role assumption".to_string()))?;
                
            // Step 5: Write credentials to ~/.aws/credentials
            let home = dirs::home_dir()
                .ok_or_else(|| Error::Module("Could not determine home directory".to_string()))?;
            let credentials_path = home.join(".aws").join("credentials");
            
            if let Some(stdout) = stdout.as_mut() {
                writeln!(stdout, "Writing temporary credentials to AWS credentials file: {}", credentials_path.display())?;
            }
            
            // Read existing credentials file
            let mut credential_content = String::new();
            if credentials_path.exists() {
                std::fs::File::open(&credentials_path)
                    .map_err(|e| Error::Module(format!("Failed to open credentials file: {}", e)))?
                    .read_to_string(&mut credential_content)
                    .map_err(|e| Error::Module(format!("Failed to read credentials file: {}", e)))?;
            }
            
            // Parse and update the credentials file
            let default_profile_content = format!(
                "[default]\naws_access_key_id = {}\naws_secret_access_key = {}\naws_session_token = {}\n",
                admin_credentials.access_key_id().unwrap_or_default(),
                admin_credentials.secret_access_key().unwrap_or_default(),
                admin_credentials.session_token().unwrap_or_default()
            );
            
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
            
            // Write the updated content back to the file
            std::fs::create_dir_all(home.join(".aws"))
                .map_err(|e| Error::Module(format!("Failed to create .aws directory: {}", e)))?;
                
            std::fs::write(&credentials_path, lines.join("\n"))
                .map_err(|e| Error::Module(format!("Failed to write credentials file: {}", e)))?;
                
            if let Some(stdout) = stdout.as_mut() {
                writeln!(stdout, "Successfully obtained and stored temporary credentials. Valid for 12 hours.")
                    .map_err(|e| Error::Module(format!("Failed to write to stdout: {}", e)))?;
            }
            
            Result::<()>::Ok(())
        })?;
        
        // Return success response
        Ok(json!({
            "success": true,
            "message": "Successfully obtained temporary credentials for blueAdminAccess-DO-NOT-DELETE role"
        }))
    }
}

impl Module for AdaModule {
    fn name(&self) -> &str {
        &self.manifest.module.name
    }

    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn call(&mut self, path: &[&str], args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> Result<Value> {
        match path {
            ["ada"] => self.handle_ada(args, stdout, stderr),
            ["status"] => self.handle_status(args, stdout, stderr),
            ["provision"] => self.handle_provision(args, stdout, stderr),
            ["creds"] => self.handle_creds(args, stdout, stderr),
            _ => Err(Error::MethodNotFound(path.join(" ")))
        }
    }
}

#[no_mangle]
pub fn create_module() -> Box<dyn Module> {
    Box::new(AdaModule::new().expect("Failed to create ada module"))
}
