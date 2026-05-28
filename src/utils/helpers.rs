use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

static SUDO_PASSWORD: Mutex<Option<String>> = Mutex::new(None);

pub fn set_sudo_password(pw: String) {
    if let Ok(mut guard) = SUDO_PASSWORD.lock() {
        *guard = Some(pw);
    }
}

pub fn clear_sudo_password() {
    if let Ok(mut guard) = SUDO_PASSWORD.lock() {
        *guard = None;
    }
}

pub fn has_sudo_password() -> bool {
    SUDO_PASSWORD.lock().map(|g| g.is_some()).unwrap_or(false)
}

pub struct CmdResult {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn run_cmd(cmd: &[&str], timeout_secs: u64) -> CmdResult {
    let mut child = match Command::new(cmd[0])
        .args(&cmd[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return CmdResult {
                code: -2,
                stdout: String::new(),
                stderr: format!("Command failed: {}", e),
            }
        }
    };

    let (tx_out, rx_out) = mpsc::channel();
    let (tx_err, rx_err) = mpsc::channel();

    if let Some(mut out) = child.stdout.take() {
        std::thread::spawn(move || {
            let mut s = String::new();
            let _ = out.read_to_string(&mut s);
            let _ = tx_out.send(s);
        });
    } else {
        let _ = tx_out.send(String::new());
    }

    if let Some(mut err) = child.stderr.take() {
        std::thread::spawn(move || {
            let mut s = String::new();
            let _ = err.read_to_string(&mut s);
            let _ = tx_err.send(s);
        });
    } else {
        let _ = tx_err.send(String::new());
    }

    let timeout = Duration::from_secs(timeout_secs);
    let start = Instant::now();
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    break -4;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                let _ = child.kill();
                return CmdResult {
                    code: -3,
                    stdout: String::new(),
                    stderr: format!("Failed to wait: {}", e),
                };
            }
        }
    };

    CmdResult {
        code,
        stdout: rx_out.recv().unwrap_or_default(),
        stderr: rx_err.recv().unwrap_or_default(),
    }
}

pub fn run_as_root(cmd: &[&str], timeout_secs: u64) -> CmdResult {
    let pw = SUDO_PASSWORD.lock().unwrap_or_else(|e| e.into_inner());
    let password = match pw.as_ref() {
        Some(p) => p.clone(),
        None => {
            return CmdResult {
                code: -1,
                stdout: String::new(),
                stderr: "No sudo password set".to_string(),
            }
        }
    };
    drop(pw);

    let mut child = match Command::new("sudo")
        .arg("-S")
        .args(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return CmdResult {
                code: -2,
                stdout: String::new(),
                stderr: format!("Failed to spawn sudo: {}", e),
            }
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(format!("{}\n", password).as_bytes());
    }

    let (tx_out, rx_out) = mpsc::channel();
    let (tx_err, rx_err) = mpsc::channel();

    if let Some(mut out) = child.stdout.take() {
        std::thread::spawn(move || {
            let mut s = String::new();
            let _ = out.read_to_string(&mut s);
            let _ = tx_out.send(s);
        });
    } else {
        let _ = tx_out.send(String::new());
    }

    if let Some(mut err) = child.stderr.take() {
        std::thread::spawn(move || {
            let mut s = String::new();
            let _ = err.read_to_string(&mut s);
            let _ = tx_err.send(s);
        });
    } else {
        let _ = tx_err.send(String::new());
    }

    let timeout = Duration::from_secs(timeout_secs);
    let start = Instant::now();
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    break -4;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                let _ = child.kill();
                return CmdResult {
                    code: -3,
                    stdout: String::new(),
                    stderr: format!("Failed to wait: {}", e),
                };
            }
        }
    };

    let stderr_str = rx_err.recv().unwrap_or_default();
    if stderr_str.to_lowercase().contains("incorrect password") {
        clear_sudo_password();
    }

    CmdResult {
        code,
        stdout: rx_out.recv().unwrap_or_default(),
        stderr: stderr_str,
    }
}

pub fn run_as_root_live<F: FnMut(String) + Send + 'static>(
    cmd: &[&str],
    timeout_secs: u64,
    callback: F,
) -> CmdResult {
    let pw = SUDO_PASSWORD.lock().unwrap_or_else(|e| e.into_inner());
    let password = match pw.as_ref() {
        Some(p) => p.clone(),
        None => {
            return CmdResult {
                code: -1,
                stdout: String::new(),
                stderr: "No sudo password set".to_string(),
            }
        }
    };
    drop(pw);

    let mut child = match Command::new("sudo")
        .arg("-S")
        .args(cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return CmdResult {
                code: -2,
                stdout: String::new(),
                stderr: format!("Failed to spawn sudo: {}", e),
            }
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(format!("{}\n", password).as_bytes());
    }

    let (tx_out, rx_out) = mpsc::channel::<Vec<String>>();
    let (tx_err, rx_err) = mpsc::channel::<String>();
    let (tx_done, rx_done) = mpsc::channel::<()>();

    if let Some(stdout) = child.stdout.take() {
        let mut cb = callback;
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            let mut lines = Vec::new();
            for line in reader.lines() {
                if let Ok(l) = line {
                    cb(l.clone());
                    lines.push(l);
                }
            }
            let _ = tx_out.send(lines);
        });
    } else {
        let _ = tx_out.send(Vec::new());
    }

    if let Some(mut err) = child.stderr.take() {
        std::thread::spawn(move || {
            let mut s = String::new();
            let _ = err.read_to_string(&mut s);
            let _ = tx_err.send(s);
        });
    } else {
        let _ = tx_err.send(String::new());
    }

    std::thread::spawn(move || {
        let _ = child.wait();
        let _ = tx_done.send(());
    });

    let timeout = Duration::from_secs(timeout_secs);
    let finished = match rx_done.recv_timeout(timeout) {
        Ok(()) => true,
        Err(_) => false,
    };

    let stdout_lines = rx_out.recv().unwrap_or_default();
    let stderr_str = rx_err.recv().unwrap_or_default();

    if stderr_str.to_lowercase().contains("incorrect password") {
        clear_sudo_password();
    }

    if !finished {
        return CmdResult {
            code: -4,
            stdout: stdout_lines.join("\n"),
            stderr: format!("Command timed out after {} seconds\n{}", timeout_secs, stderr_str),
        };
    }

    CmdResult {
        code: 0,
        stdout: stdout_lines.join("\n"),
        stderr: stderr_str,
    }
}

pub fn check_command(name: &str) -> bool {
    let path = std::env::var("PATH").unwrap_or_default();
    for dir in path.split(':') {
        let full = format!("{}/{}", dir, name);
        if std::path::Path::new(&full).is_file() {
            return true;
        }
    }
    false
}
