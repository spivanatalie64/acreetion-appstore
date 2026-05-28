use super::{Backend, BackendResult, PackageInfo};
use crate::utils::helpers::{self, check_command, run_as_root};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct PacmanBackend {
    enabled: AtomicBool,
}

impl PacmanBackend {
    pub fn new() -> Self {
        Self { enabled: AtomicBool::new(check_command("pacman")) }
    }
}

impl Backend for PacmanBackend {
    fn name(&self) -> &str { "pacman" }
    fn icon(&self) -> &str { "system-software-install" }
    fn enabled(&self) -> bool { self.enabled.load(Ordering::Relaxed) }
    fn set_enabled(&self, val: bool) { self.enabled.store(val, Ordering::Relaxed); }

    fn refresh(&self) { let _ = run_as_root(&["pacman", "-Sy"], 120); }

    fn search(&self, query: &str) -> Vec<PackageInfo> {
        let r = helpers::run_cmd(&["pacman", "-Ss", query], 120);
        if r.code != 0 { return vec![]; }
        let mut results = Vec::new();
        let mut current: Option<PackageInfo> = None;
        for line in r.stdout.lines() {
            if line.starts_with(' ') && !line.trim().is_empty() {
                if let Some(ref mut p) = current { p.description = line.trim().to_string(); }
            } else if !line.is_empty() {
                if let Some(p) = current.take() { results.push(p); }
                if let Some((repo, rest)) = line.split_once('/') {
                    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
                    if !parts.is_empty() {
                        current = Some(PackageInfo {
                            name: parts[0].to_string(),
                            version: if parts.len() > 1 { parts[1].trim().to_string() } else { String::new() },
                            repo: repo.to_string(), backend: "pacman".into(), backend_key: "pacman".into(),
                            ..PackageInfo::new("")
                        });
                    }
                }
            }
        }
        if let Some(p) = current { results.push(p); }
        results
    }

    fn install(&self, name: &str, version: Option<&str>) -> BackendResult {
        match version {
            Some(v) => { let pv = format!("{}={}", name, v);
                let r = run_as_root(&["pacman", "-S", "--noconfirm", &pv], 300);
                (r.code, r.stdout, r.stderr) }
            None => { let r = run_as_root(&["pacman", "-S", "--noconfirm", name], 300);
                (r.code, r.stdout, r.stderr) }
        }
    }

    fn remove(&self, name: &str) -> BackendResult {
        let r = run_as_root(&["pacman", "-R", "--noconfirm", name], 120);
        (r.code, r.stdout, r.stderr)
    }

    fn list_installed(&self) -> Vec<PackageInfo> {
        let r = helpers::run_cmd(&["pacman", "-Q"], 60);
        if r.code != 0 { return vec![]; }
        r.stdout.lines().filter_map(|line| {
            let mut parts = line.splitn(2, ' ');
            let name = parts.next()?.to_string();
            let ver = parts.next().unwrap_or("").to_string();
            Some(PackageInfo {
                name, version: ver.clone(), installed_version: ver,
                backend: "pacman".into(), backend_key: "pacman".into(),
                ..PackageInfo::new("")
            })
        }).collect()
    }

    fn check_updates(&self) -> Vec<PackageInfo> {
        let r = helpers::run_cmd(&["pacman", "-Qu"], 60);
        if r.code != 0 && r.code != 1 { return vec![]; }
        r.stdout.lines().filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                Some(PackageInfo {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    installed_version: parts.get(2).unwrap_or(&"").trim().to_string(),
                    backend: "pacman".into(), backend_key: "pacman".into(),
                    ..PackageInfo::new("")
                })
            } else { None }
        }).collect()
    }

    fn get_versions(&self, name: &str) -> Vec<String> {
        let r = helpers::run_cmd(&["pacman", "-Si", name], 30);
        if r.code != 0 { return vec![]; }
        for line in r.stdout.lines() {
            if let Some((key, val)) = line.split_once(':') {
                if key.trim() == "Version" { return vec![val.trim().to_string()]; }
            }
        }
        vec![]
    }

    fn get_info(&self, name: &str) -> PackageInfo {
        let mut pi = PackageInfo::new(name);
        pi.backend = "pacman".into(); pi.backend_key = "pacman".into();
        let r = helpers::run_cmd(&["pacman", "-Qi", name], 30);
        let stdout = if r.code == 0 { r.stdout } else {
            let r2 = helpers::run_cmd(&["pacman", "-Si", name], 30);
            if r2.code == 0 { r2.stdout } else { return pi; }
        };
        for line in stdout.lines() {
            if let Some((key, val)) = line.split_once(':') {
                match key.trim() {
                    "Version" => { pi.version = val.trim().to_string(); pi.installed_version = val.trim().to_string(); }
                    "Installed Size" => pi.size = val.trim().to_string(),
                    "Description" => pi.description = val.trim().to_string(),
                    "URL" => pi.url = val.trim().to_string(),
                    "Licenses" => pi.license = val.trim().to_string(),
                    "Repository" | "Repo" => pi.repo = val.trim().to_string(),
                    _ => {}
                }
            }
        }
        pi
    }

    fn install_self(&self) -> BackendResult { (0, String::new(), String::new()) }
}
