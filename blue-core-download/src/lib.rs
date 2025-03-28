use blue_core::prelude::*;
use serde::{Serialize, Deserialize};
use futures_util::StreamExt;
use std::io::Write;
use std::time::Instant;

#[derive(Serialize, Deserialize)]
struct DownloadStats {
    progress: f64,
    speed: f64,
    downloaded: u64,
    total: u64,
    elapsed: f64,
}

pub struct CoreDownload;

impl CoreDownload {
    pub fn new() -> Self {
        Self
    }

    async fn download(&self, url: &str, output_dir: &std::path::Path, stdout: &mut dyn Write) -> Result<()> {
        // Create client with relaxed TLS settings
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| Error::Module(format!("Failed to create HTTP client: {}", e)))?;
        
        let response = client.get(url).send().await
            .map_err(|e| Error::Module(format!("Failed to start download: {}", e)))?;
        
        let total_size = response.content_length().unwrap_or(0);
        let filename = url.split('/').last().unwrap_or("download");
        let mut file = std::fs::File::create(output_dir.join(filename))
            .map_err(|e| Error::Module(format!("Failed to create output file: {}", e)))?;
        
        let mut downloaded = 0;
        let start_time = Instant::now();
        let mut last_update = start_time;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| Error::Module(format!("Download error: {}", e)))?;
            file.write_all(&chunk)
                .map_err(|e| Error::Module(format!("Failed to write to file: {}", e)))?;
            downloaded += chunk.len() as u64;
            
            // Update progress every second
            let now = Instant::now();
            if now.duration_since(last_update).as_secs() >= 1 {
                let elapsed = now.duration_since(start_time).as_secs_f64();
                let stats = DownloadStats {
                    progress: if total_size > 0 { downloaded as f64 / total_size as f64 * 100.0 } else { 0.0 },
                    speed: downloaded as f64 / elapsed,
                    downloaded,
                    total: total_size,
                    elapsed,
                };
                writeln!(stdout, "{}", serde_json::to_string(&stats)
                    .map_err(|e| Error::Module(format!("Failed to serialize stats: {}", e)))?)
                    .map_err(|e| Error::Module(format!("Failed to write to stdout: {}", e)))?;
                stdout.flush()
                    .map_err(|e| Error::Module(format!("Failed to flush stdout: {}", e)))?;
                last_update = now;
            }
        }

        Ok(())
    }
}

impl Module for CoreDownload {
    fn name(&self) -> &str {
        "core-download"
    }

    fn manifest(&self) -> &ModuleManifest {
        // Core modules don't need manifests
        panic!("Core modules don't use manifests");
    }

    fn call(&mut self, path: &[&str], args: Value, stdout: Option<&mut dyn Write>, _stderr: Option<&mut dyn Write>) -> Result<Value> {
        match path {
            ["get"] => {
                let url = args.get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| Error::Module("Missing URL".into()))?;
                
                let output_dir = args.get("output_dir")
                    .and_then(|v| v.as_str())
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| std::path::PathBuf::from("."));

                // Create tokio runtime for async download
                tokio::runtime::Runtime::new()
                    .map_err(|e| Error::Module(format!("Failed to create runtime: {}", e)))?
                    .block_on(self.download(url, &output_dir, stdout.unwrap()))?;

                Ok(Value::Null)
            }
            _ => Err(Error::MethodNotFound(path.join(" ")))
        }
    }
}

#[no_mangle]
pub fn create_module() -> Box<dyn Module> {
    Box::new(CoreDownload::new())
}
