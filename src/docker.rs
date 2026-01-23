use anyhow::{Context, Result, bail};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write as IoWrite};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::output::Output;

pub fn ensure_available() -> Result<()> {
    let status = Command::new("docker")
        .arg("version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => bail!("Docker is installed but not responding. Is the daemon running?"),
        Err(_) => bail!("Docker not found in PATH."),
    }
}

pub fn user_args() -> Vec<String> {
    #[cfg(unix)]
    {
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        vec!["--user".to_string(), format!("{uid}:{gid}")]
    }
    #[cfg(not(unix))]
    {
        Vec::new()
    }
}

pub fn user_flag() -> String {
    #[cfg(unix)]
    {
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        format!("--user {uid}:{gid}")
    }
    #[cfg(not(unix))]
    {
        String::new()
    }
}

pub fn run_with_logging(command: &mut Command, log_path: &Path, output: &Output) -> Result<bool> {
    if output.verbose {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = command.spawn().context("Failed to start Docker command")?;
        let stdout = child.stdout.take().context("Missing stdout")?;
        let stderr = child.stderr.take().context("Missing stderr")?;

        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .context("Failed to open log file")?;
        let log = Arc::new(Mutex::new(log_file));

        let out_log = Arc::clone(&log);
        let out_handle = thread::spawn(move || stream_output(stdout, io::stdout(), out_log));
        let err_log = Arc::clone(&log);
        let err_handle = thread::spawn(move || stream_output(stderr, io::stderr(), err_log));

        let status = child.wait().context("Failed to wait for command")?;
        let _ = out_handle.join();
        let _ = err_handle.join();
        Ok(status.success())
    } else {
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .context("Failed to open log file")?;
        let log_err = log_file.try_clone().context("Failed to clone log file")?;
        command
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(log_err));
        let status = command.status().context("Failed to run Docker command")?;
        Ok(status.success())
    }
}

fn stream_output<R: Read + Send + 'static>(
    mut reader: R,
    mut writer: impl IoWrite + Send + 'static,
    log: Arc<Mutex<File>>,
) -> io::Result<()> {
    let mut buffer = [0u8; 8192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        {
            let mut file = log
                .lock()
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "Log file lock poisoned"))?;
            file.write_all(&buffer[..count])?;
        }
        writer.write_all(&buffer[..count])?;
        writer.flush()?;
    }
    Ok(())
}
