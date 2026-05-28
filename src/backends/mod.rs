pub mod pacman;
pub mod yay;
pub mod paru;
pub mod flatpak;
pub mod snap;
pub mod appimage;

use std::collections::HashMap;

pub type BackendResult = (i32, String, String);

#[derive(Clone, Debug)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub repo: String,
    pub backend: String,
    pub backend_key: String,
    pub size: String,
    pub url: String,
    pub license: String,
    pub installed_version: String,
    pub available_versions: Vec<String>,
}

impl PackageInfo {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: String::new(),
            description: String::new(),
            repo: String::new(),
            backend: String::new(),
            backend_key: String::new(),
            size: String::new(),
            url: String::new(),
            license: String::new(),
            installed_version: String::new(),
            available_versions: Vec::new(),
        }
    }

    pub fn is_installed(&self) -> bool {
        !self.installed_version.is_empty()
    }

    pub fn has_update(&self) -> bool {
        self.is_installed() && self.installed_version != self.version
    }
}

pub trait Backend: Send + Sync {
    fn name(&self) -> &str;
    fn icon(&self) -> &str;
    fn enabled(&self) -> bool;
    fn set_enabled(&self, val: bool);
    fn refresh(&self);
    fn search(&self, query: &str) -> Vec<PackageInfo>;
    fn install(&self, name: &str, version: Option<&str>) -> BackendResult;
    fn remove(&self, name: &str) -> BackendResult;
    fn list_installed(&self) -> Vec<PackageInfo>;
    fn check_updates(&self) -> Vec<PackageInfo>;
    fn get_versions(&self, name: &str) -> Vec<String>;
    fn get_info(&self, name: &str) -> PackageInfo;
    fn install_self(&self) -> BackendResult;
}

pub struct BackendManager {
    pub backends: HashMap<String, Box<dyn Backend>>,
}

impl BackendManager {
    pub fn new() -> Self {
        let mut bm = Self { backends: HashMap::new() };
        bm.backends.insert("pacman".into(), Box::new(pacman::PacmanBackend::new()));
        bm.backends.insert("yay".into(), Box::new(yay::YayBackend::new()));
        bm.backends.insert("paru".into(), Box::new(paru::ParuBackend::new()));
        bm.backends.insert("flatpak".into(), Box::new(flatpak::FlatpakBackend::new()));
        bm.backends.insert("snap".into(), Box::new(snap::SnapBackend::new()));
        bm.backends.insert("appimage".into(), Box::new(appimage::AppImageBackend::new()));
        bm
    }

    pub fn enabled_backends(&self) -> Vec<(&str, &dyn Backend)> {
        self.backends.iter()
            .filter(|(_, b)| b.enabled())
            .map(|(k, b)| (k.as_str(), b.as_ref()))
            .collect()
    }

    pub fn find_backend(&self, pkg: &PackageInfo) -> Option<&dyn Backend> {
        for (key, backend) in &self.backends {
            if backend.enabled() && (backend.name() == pkg.backend || key == &pkg.backend_key) {
                return Some(backend.as_ref());
            }
        }
        self.backends.get(&pkg.backend_key).map(|b| b.as_ref())
    }

    pub fn search_all(&self, query: &str) -> Vec<PackageInfo> {
        let mut results = Vec::new();
        for (_, backend) in &self.backends {
            if backend.enabled() {
                results.extend(backend.search(query));
            }
        }
        results
    }

    pub fn refresh_all(&self) {
        for (_, backend) in &self.backends {
            if backend.enabled() {
                backend.refresh();
            }
        }
    }
}
