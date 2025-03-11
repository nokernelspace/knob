use crate::types::*;
use std::env::consts;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::time::*;
use std::time::{SystemTime, UNIX_EPOCH};

/// ! Change the working directory of the build process
pub fn cd(relative: &str) {
    let base_path = std::env::current_dir().unwrap();
    let base_path = base_path.join(relative);

    //println!("CD: {:?}", base_path);
    let absolute = std::fs::canonicalize(&base_path)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    std::env::set_current_dir(&absolute).unwrap();
}

///! Check if binary exists
pub fn bin_exists(bin: &str) -> bool {
    match Command::new(bin).output() {
        Ok(status) => return true,
        Err(e) => return false,
    }
}

/// ! Execute a Binary with Command Line Arguments
pub fn execute(
    binary: &str,
    args: &Vec<String>,
    echo: bool,
    stream: bool,
) -> std::result::Result<String, String> {
    let mut _command = String::new();
    _command.push_str(binary);
    for a in args {
        _command.push_str(" ");
        _command.push_str(a);
    }
    if echo {
        println!("{}", _command);
    }

    if !stream {
        match Command::new(binary).args(args).output() {
            Ok(output) => {
                let ok = output.status.success();
                let code = output.status.code().unwrap();

                let stdout = String::from_utf8(output.stdout).unwrap();
                let stderr = String::from_utf8(output.stderr).unwrap();

                if ok {
                    Ok(stdout)
                } else {
                    Err(stderr)
                }
            }
            Err(e) => Err(e.to_string()),
        }
    } else {
        let mut command = Command::new(binary);
        let process = command.args(args);
        process.stdout(Stdio::inherit());
        process.stderr(Stdio::inherit());
        let mut handle = {
            match process.spawn() {
                Ok(handle) => handle,
                Err(e) => {
                    return Err(e.to_string());
                }
            }
        };

        loop {
            let status = handle.wait().unwrap();
            if status.success() {
                return Ok("".to_string());
            } else {
                return Err(format!(
                    "{} exited with code {}",
                    binary,
                    status.code().unwrap()
                ));
            }
        }
    }
}

pub fn now() -> i64 {
    let unix_timestamp: i64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    unix_timestamp
}

pub fn since(timestamp1: u64, timestamp2: u64) -> u64 {
    // Convert UNIX timestamps to SystemTime
    let time1 = UNIX_EPOCH + std::time::Duration::from_secs(timestamp1);
    let time2 = UNIX_EPOCH + std::time::Duration::from_secs(timestamp2);

    // Calculate the difference
    let difference = if time1 > time2 {
        time1
            .duration_since(time2)
            .unwrap_or(std::time::Duration::from_secs(0))
    } else {
        time2
            .duration_since(time1)
            .unwrap_or(std::time::Duration::from_secs(0))
    };

    difference.as_secs()
}

pub fn cwd() -> Box<Path> {
    let base_path = std::env::current_dir().unwrap();
    std::fs::canonicalize(&base_path).unwrap().into_boxed_path()
}

pub fn rm(path: &Path) {
    if path.exists() {
        if path.is_file() {
            std::fs::remove_file(path).unwrap();
        } else if path.is_dir() {
            std::fs::remove_dir_all(path).unwrap();
        }
    }
}
pub fn mkdir(path: &Path) {
    if !path.exists() {
        std::fs::create_dir(path).unwrap();
    }
}

pub fn canonicalize(rel_path: &str) -> Box<Path> {
    std::fs::canonicalize(rel_path).unwrap().into_boxed_path()
}

pub fn last_modified(path: &String) -> i32 {
    let metadata = std::fs::metadata(path).unwrap();
    let modified_time = metadata.modified().unwrap();
    let duration = modified_time
        .duration_since(UNIX_EPOCH)
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "Modified time is before UNIX epoch"))
        .unwrap();
    duration.as_secs() as i32
}
