use super::{Backend, PackageInfo};
use std::path::Path;

pub struct AppImageBackend {
    pub enabled: bool,
}

impl AppImageBackend {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    fn discover_local(&self) -> Vec<PackageInfo> {
        let mut results = Vec::new();
        let dirs = vec![
            format!("{}/Applications", std::env::var("HOME").unwrap_or_default()),
            format!("{}/.local/bin", std::env::var("HOME").unwrap_or_default()),
        ];
        for dir in &dirs {
            let path = Path::new(dir);
            if path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let fname = entry.file_name().to_string_lossy().to_string();
                        if fname.ends_with(".AppImage") || fname.ends_with(".appimage") {
                            let name = fname.replace(".AppImage", "").replace(".appimage", "");
                            let size = match entry.metadata() {
                                Ok(m) => format_size(m.len()),
                                Err(_) => String::new(),
                            };
                            results.push(PackageInfo {
                                name, version: "local".to_string(),
                                description: format!("AppImage in {}", dir),
                                size,
                                installed_version: "local".to_string(),
                                backend: "AppImage".to_string(), backend_key: "appimage".to_string(),
                                ..PackageInfo::new("")
                            });
                        }
                    }
                }
            }
        }
        results
    }
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    for unit in UNITS {
        if size < 1024.0 {
            return format!("{:.1} {}", size, unit);
        }
        size /= 1024.0;
    }
    format!("{:.1} TB", size)
}

impl Backend for AppImageBackend {
    fn name(&self) -> &str { "AppImage" }
    fn icon(&self) -> &str { "application-x-executable" }
    fn enabled(&self) -> bool { true }
    fn set_enabled(&self, _val: bool) {}

    fn refresh(&self) {}

    fn search(&self, query: &str) -> Vec<PackageInfo> {
        let all = self.discover_local();
        if query.is_empty() { return all; }
        let q = query.to_lowercase();
        all.into_iter().filter(|p| p.name.to_lowercase().contains(&q)).collect()
    }

    fn install(&self, _name: &str, _version: Option<&str>) -> (i32, String, String) {
        (1, "".to_string(), "Download AppImages manually to ~/Applications/".to_string())
    }

    fn remove(&self, name: &str) -> (i32, String, String) {
        let dirs = vec![
            format!("{}/Applications", std::env::var("HOME").unwrap_or_default()),
            format!("{}/.local/bin", std::env::var("HOME").unwrap_or_default()),
        ];
        for dir in &dirs {
            let path = Path::new(dir);
            if path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let fname = entry.file_name().to_string_lossy().to_string();
                        let base = fname.replace(".AppImage", "").replace(".appimage", "");
                        if base == name {
                            let p = entry.path();
                            let _ = std::fs::remove_file(&p);
                            return (0, format!("Removed {}", p.display()), String::new());
                        }
                    }
                }
            }
        }
        (1, String::new(), format!("AppImage '{}' not found", name))
    }

    fn list_installed(&self) -> Vec<PackageInfo> {
        self.discover_local()
    }

    fn check_updates(&self) -> Vec<PackageInfo> {
        vec![]
    }

    fn get_versions(&self, _name: &str) -> Vec<String> {
        vec![]
    }

    fn get_info(&self, name: &str) -> PackageInfo {
        let all = self.discover_local();
        all.into_iter().find(|p| p.name == name)
            .unwrap_or_else(|| PackageInfo {
                name: name.to_string(),
                backend: "AppImage".to_string(), backend_key: "appimage".to_string(),
                ..PackageInfo::new("")
            })
    }

    fn install_self(&self) -> (i32, String, String) {
        (0, String::new(), String::new())
    }
}
