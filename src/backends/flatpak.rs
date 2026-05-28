use super::{Backend, BackendResult, PackageInfo};
use crate::utils::helpers::{self, check_command, run_as_root};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct FlatpakBackend { enabled: AtomicBool }

impl FlatpakBackend {
    pub fn new() -> Self { Self { enabled: AtomicBool::new(check_command("flatpak")) } }
}

impl Backend for FlatpakBackend {
    fn name(&self) -> &str { "Flatpak" }
    fn icon(&self) -> &str { "package-x-generic" }
    fn enabled(&self) -> bool { self.enabled.load(Ordering::Relaxed) }
    fn set_enabled(&self, val: bool) { self.enabled.store(val, Ordering::Relaxed); }

    fn refresh(&self) { if self.enabled() { let _ = helpers::run_cmd(&["flatpak", "update", "--appstream"], 60); } }

    fn search(&self, query: &str) -> Vec<PackageInfo> {
        if !self.enabled() { return vec![]; }
        let r = helpers::run_cmd(&["flatpak", "search", query], 60);
        if r.code != 0 { return vec![]; }
        r.stdout.lines().skip(1).filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                Some(PackageInfo {
                    name: parts[0].to_string(), version: parts[1].to_string(),
                    description: parts[2].to_string(),
                    backend: "Flatpak".into(), backend_key: "flatpak".into(),
                    ..PackageInfo::new("")
                })
            } else { None }
        }).collect()
    }

    fn install(&self, name: &str, version: Option<&str>) -> BackendResult {
        match version {
            Some(v) => { let pv = format!("{}//{}", name, v);
                let r = run_as_root(&["flatpak", "install", "-y", &pv], 300);
                (r.code, r.stdout, r.stderr) }
            None => { let r = run_as_root(&["flatpak", "install", "-y", name], 300);
                (r.code, r.stdout, r.stderr) }
        }
    }

    fn remove(&self, name: &str) -> BackendResult {
        let r = run_as_root(&["flatpak", "uninstall", "-y", name], 120);
        (r.code, r.stdout, r.stderr)
    }

    fn list_installed(&self) -> Vec<PackageInfo> {
        if !self.enabled() { return vec![]; }
        let r = helpers::run_cmd(&["flatpak", "list", "--columns=application,version"], 30);
        if r.code != 0 { return vec![]; }
        r.stdout.lines().skip(1).filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.is_empty() || parts[0].is_empty() { return None; }
            let ver = if parts.len() > 1 { parts[1].to_string() } else { String::new() };
            Some(PackageInfo { name: parts[0].to_string(), version: ver.clone(), installed_version: ver,
                backend: "Flatpak".into(), backend_key: "flatpak".into(), ..PackageInfo::new("") })
        }).collect()
    }

    fn check_updates(&self) -> Vec<PackageInfo> {
        if !self.enabled() { return vec![]; }
        let r = helpers::run_cmd(&["flatpak", "remote-ls", "--updates", "--columns=application,version"], 30);
        if r.code != 0 { return vec![]; }
        r.stdout.lines().skip(1).filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.is_empty() || parts[0].is_empty() { return None; }
            Some(PackageInfo { name: parts[0].to_string(),
                version: if parts.len() > 1 { parts[1].to_string() } else { String::new() },
                backend: "Flatpak".into(), backend_key: "flatpak".into(), ..PackageInfo::new("") })
        }).collect()
    }

    fn get_versions(&self, name: &str) -> Vec<String> {
        let r = helpers::run_cmd(&["flatpak", "remote-ls", "--columns=application,branch", name], 30);
        if r.code != 0 { return vec!["stable".to_string()]; }
        let mut v: Vec<String> = r.stdout.lines().skip(1).filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 && parts[0].trim() == name { Some(parts[1].trim().to_string()) } else { None }
        }).collect();
        if v.is_empty() { v.push("stable".to_string()); }
        v
    }

    fn get_info(&self, name: &str) -> PackageInfo {
        let mut pi = PackageInfo::new(name);
        pi.backend = "Flatpak".into(); pi.backend_key = "flatpak".into();
        let r = helpers::run_cmd(&["flatpak", "info", name], 30);
        if r.code == 0 {
            for line in r.stdout.lines() {
                if let Some((key, val)) = line.split_once(':') {
                    match key.trim() {
                        "Version" => pi.version = val.trim().to_string(),
                        "Installed" => pi.installed_version = val.trim().to_string(),
                        "Description" => pi.description = val.trim().to_string(),
                        "URL" => pi.url = val.trim().to_string(),
                        "License" => pi.license = val.trim().to_string(),
                        "Size" => pi.size = val.trim().to_string(),
                        _ => {}
                    }
                }
            }
        }
        pi
    }

    fn install_self(&self) -> BackendResult {
        let r = run_as_root(&["pacman", "-S", "--noconfirm", "flatpak"], 300);
        if r.code == 0 { self.set_enabled(true); }
        (r.code, r.stdout, r.stderr)
    }
}
