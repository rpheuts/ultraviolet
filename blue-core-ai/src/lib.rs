use blue_core::prelude::*;
use aws_sdk_bedrockruntime::primitives::Blob;
use aws_sdk_bedrockruntime::types::ResponseStream;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tokio::runtime::Runtime;

const DEFAULT_MODEL: &str = "anthropic.claude-3-5-sonnet-20241022-v2:0";

pub struct CoreAiModule {
    runtime: Runtime,
    client: aws_sdk_bedrockruntime::Client,
    manifest: ModuleManifest,
}

impl CoreAiModule {
    pub fn new() -> Result<Self> {
        let runtime = Runtime::new()
            .map_err(|e| Error::Module(format!("Failed to create runtime: {}", e)))?;
        
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|e| Error::Module(format!("Failed to get home directory: {}", e)))?;

        let lib_path = home.join(".blue/modules");
        let manifest = ModuleManifest::load(lib_path.join("blue").join("core-ai").join("manifest.toml"))?;
        
        // Initialize AWS client
        let client = runtime.block_on(async move {
            let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(aws_config::Region::new("us-west-2"))  // Default to us-west-2
                .load()
                .await;

            let client = aws_sdk_bedrockruntime::Client::new(&config);
            std::result::Result::Ok::<aws_sdk_bedrockruntime::Client, blue_core::Error>(client)
        })?;

        Ok(Self { runtime, client, manifest })
    }

    fn build_prompt(prompt: &str, files: &[String]) -> Result<String> {
        let mut full_prompt = prompt.to_string();

        if !files.is_empty() {
            full_prompt.push_str("\n\nI'm also providing the contents of the following files for context:\n");
            
            for file in files {
                let path = PathBuf::from(file);
                let content = fs::read_to_string(&path)
                    .map_err(|e| Error::Module(format!("Failed to read file {}: {}", file, e)))?;
                let filename = path.file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("unknown");
                
                full_prompt.push_str(&format!("\n=== File: {} ===\n{}\n", filename, content));
            }

            full_prompt.push_str("\nPlease consider these files in your response.");
        }

        Ok(full_prompt)
    }

    async fn invoke_stream(&self, prompt: &str, stdout: Option<&mut dyn Write>) -> Result<String> {
        let request = json!({
            "anthropic_version": "bedrock-2023-05-31",
            "max_tokens": 4096,
            "messages": [{
                "role": "user",
                "content": [{"type": "text", "text": prompt}]
            }]
        });

        let mut stream = self.client.invoke_model_with_response_stream()
            .model_id(DEFAULT_MODEL)
            .body(Blob::new(request.to_string()))
            .content_type("application/json")
            .accept("application/json")
            .send()
            .await
            .map_err(|e| {
                println!("Bedrock streaming error details: {:?}", e);
                Error::Module(format!("Failed to invoke Bedrock model with streaming: {} (Make sure AWS credentials are configured in ~/.aws/credentials and have Bedrock permissions)", e))
            })?
            .body;

        let mut response = String::new();
        let mut stdout_ref = stdout;

        while let Some(event) = stream.recv().await
            .map_err(|e| Error::Module(format!("Failed to receive stream chunk: {}", e)))? {
            if let ResponseStream::Chunk(data) = event {
                let bytes = data.bytes
                    .ok_or_else(|| Error::Module("Missing chunk bytes".into()))?;
                let chunk: Value = serde_json::from_slice(bytes.as_ref())
                    .map_err(|e| Error::Module(format!("Failed to parse chunk: {}", e)))?;
                
                if let Some(text) = chunk["delta"]["text"].as_str() {
                    response.push_str(text);
                    if let Some(w) = stdout_ref.as_mut() {
                        w.write_all(text.as_bytes())?;
                        w.flush()?;
                    }
                }
            }
        }

        Ok(response)
    }

    async fn invoke_single(&self, prompt: &str) -> Result<String> {
        let request = json!({
            "anthropic_version": "bedrock-2023-05-31",
            "max_tokens": 4096,
            "messages": [{
                "role": "user",
                "content": [{"type": "text", "text": prompt}]
            }]
        });

        let response = self.client.invoke_model()
            .model_id(DEFAULT_MODEL)
            .body(Blob::new(request.to_string()))
            .content_type("application/json")
            .accept("application/json")
            .send()
            .await
            .map_err(|e| {
                println!("Bedrock error details: {:?}", e);
                Error::Module(format!("Failed to invoke Bedrock model: {} (Make sure AWS credentials are configured in ~/.aws/credentials and have Bedrock permissions)", e))
            })?;

        let response_json: Value = serde_json::from_slice(response.body.as_ref())
            .map_err(|e| Error::Module(format!("Failed to parse response: {}", e)))?;
        
        response_json["content"][0]["text"].as_str()
            .map(String::from)
            .ok_or_else(|| Error::Module("Invalid response format".into()))
    }

    pub fn handle_ask(&mut self, args: Value, stdout: Option<&mut dyn Write>, _stderr: Option<&mut dyn Write>) -> Result<Value> {
        // Extract arguments
        let args = args.as_object()
            .ok_or_else(|| Error::Module("Invalid arguments".into()))?;

        let prompt = args.get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Module("Missing required argument: prompt".into()))?;

        let files: Vec<String> = args.get("files")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let stream = args.get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Build the full prompt
        let full_prompt = Self::build_prompt(prompt, &files)?;

        // Get response
        let response = if stream {
            self.runtime.block_on(async {
                self.invoke_stream(&full_prompt, stdout).await
            })?
        } else {
            self.runtime.block_on(async {
                self.invoke_single(&full_prompt).await
            })?
        };

        Ok(json!({ "response": response }))
    }
}

impl Module for CoreAiModule {
    fn name(&self) -> &str {
        &self.manifest.module.name
    }

    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn call(&mut self, path: &[&str], args: Value, stdout: Option<&mut dyn Write>, stderr: Option<&mut dyn Write>) -> Result<Value> {
        // Validate method exists
        if self.manifest.find_method(path).is_none() {
            return Err(Error::MethodNotFound(path.join(" ")));
        }

        match path {
            ["ask"] => self.handle_ask(args, stdout, stderr),
            _ => Err(Error::MethodNotFound(path.join(" ")))
        }
    }
}

#[no_mangle]
pub fn create_module() -> Box<dyn Module> {
    Box::new(CoreAiModule::new().expect("Failed to create core-ai module"))
}
