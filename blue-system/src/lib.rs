use blue_core::prelude::*;
use serde::Serialize;
use sysinfo::{Disks, System};

#[derive(Serialize)]
struct StorageDevice {
    device: String,
    mount_point: String,
    filesystem: String,
    total_bytes: u64,
    available_bytes: u64,
    percent_used: f32,
}

#[derive(Serialize)]
struct SystemInfo {
    architecture: String,
    os_type: String,
    os_version: String,
    hostname: String,
    cpu_cores: u32,
    memory_total: u64,
    memory_available: u64,
    swap_total: u64,
    swap_free: u64,
    load_average: f32,
    cpu_usage: f32,
    storage_devices: Vec<StorageDevice>,
}

#[derive(Serialize)]
struct FormattedStorageDevice {
    device: String,
    mount_point: String,
    filesystem: String,
    total_bytes: String,
    available_bytes: String,
    percent_used: String,
}

#[derive(Serialize)]
struct FormattedSystemInfo {
    architecture: String,
    os_type: String,
    os_version: String,
    hostname: String,
    cpu_cores: u32,
    memory_total: String,
    memory_available: String,
    swap_total: String,
    swap_free: String,
    load_average: String,
    cpu_usage: String,
    storage_devices: Vec<FormattedStorageDevice>,
}

pub struct SystemModule {
    sys: System,
    manifest: ModuleManifest,
}

impl SystemModule {
    pub fn new() -> Result<Self> {
        let mut sys = System::new_all();
        sys.refresh_all();

        // Load manifest from the same directory as this file
        let manifest = ModuleManifest::load(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("manifest.toml")
        )?;

        Ok(Self { sys, manifest })
    }

    fn format_bytes(bytes: u64) -> String {
        let units = ["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < units.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.1} {}", size, units[unit_index])
    }

    fn format_percent(value: f32) -> String {
        format!("{}%", value.round() as i32)
    }

    fn get_system_info(&mut self) -> Result<SystemInfo> {
        // Refresh system information
        self.sys.refresh_all();

        // Get storage devices
        let storage_devices = Disks::new_with_refreshed_list().iter().map(|disk| {
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total.saturating_sub(available);
            let percent_used = if total > 0 {
                (used as f32 / total as f32) * 100.0
            } else {
                0.0
            };

            StorageDevice {
                device: disk.name().to_string_lossy().into_owned(),
                mount_point: disk.mount_point().to_string_lossy().into_owned(),
                filesystem: disk.file_system().to_string_lossy().into_owned(),
                total_bytes: total,
                available_bytes: available,
                percent_used,
            }
        }).collect();

        Ok(SystemInfo {
            architecture: std::env::consts::ARCH.to_string(),
            os_type: std::env::consts::OS.to_string(),
            os_version: std::env::consts::FAMILY.to_string(),
            hostname: gethostname::gethostname().to_string_lossy().into_owned(),
            cpu_cores: self.sys.cpus().len() as u32,
            memory_total: self.sys.total_memory(),
            memory_available: self.sys.free_memory(),
            swap_total: self.sys.total_swap(),
            swap_free: self.sys.free_swap(),
            load_average: self.sys.global_cpu_info().cpu_usage(),
            cpu_usage: self.sys.global_cpu_info().cpu_usage(),
            storage_devices,
        })
    }

    fn format_system_info(&mut self, info: SystemInfo) -> FormattedSystemInfo {
        FormattedSystemInfo {
            architecture: info.architecture,
            os_type: info.os_type,
            os_version: info.os_version,
            hostname: info.hostname,
            cpu_cores: info.cpu_cores,
            memory_total: Self::format_bytes(info.memory_total),
            memory_available: Self::format_bytes(info.memory_available),
            swap_total: Self::format_bytes(info.swap_total),
            swap_free: Self::format_bytes(info.swap_free),
            load_average: Self::format_percent(info.load_average),
            cpu_usage: Self::format_percent(info.cpu_usage),
            storage_devices: info.storage_devices.into_iter().map(|dev| FormattedStorageDevice {
                device: dev.device,
                mount_point: dev.mount_point,
                filesystem: dev.filesystem,
                total_bytes: Self::format_bytes(dev.total_bytes),
                available_bytes: Self::format_bytes(dev.available_bytes),
                percent_used: Self::format_percent(dev.percent_used),
            }).collect(),
        }
    }

    fn handle_stats(&mut self) -> Result<Value> {
        Ok(serde_json::to_value(self.get_system_info()?)?)
    }

    fn handle_info(&mut self) -> Result<Value> {
        let info = self.get_system_info()?;
        Ok(serde_json::to_value(self.format_system_info(info))?)
    }
}

impl Module for SystemModule {
    fn name(&self) -> &str {
        &self.manifest.module.name
    }

    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn call(&mut self, path: &[&str], _args: Value, _stdout: Option<&mut dyn Write>, _stderr: Option<&mut dyn Write>) -> Result<Value> {
        // Validate method exists
        if self.manifest.find_method(path).is_none() {
            return Err(Error::MethodNotFound(path.join(" ")));
        }

        match path {
            ["stats"] => self.handle_stats(),
            ["info"] => self.handle_info(),
            _ => Err(Error::MethodNotFound(path.join(" ")))
        }
    }
}

#[no_mangle]
pub fn create_module() -> Box<dyn Module> {
    Box::new(SystemModule::new().expect("Failed to create system module"))
}
