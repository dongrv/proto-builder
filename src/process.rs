use crate::{report::Reporter, util};
use anyhow::{bail, Context, Result};
use std::{
    io::Read,
    process::{Command, Stdio},
    sync::atomic::Ordering,
    thread,
    time::{Duration, Instant},
};

pub struct Captured {
    pub stdout: String,
    pub stderr: String,
}
pub fn run(
    command: &mut Command,
    phase: &str,
    timeout: u64,
    report: &Reporter,
) -> Result<Captured> {
    util::check_cancelled()?;
    report.detail(phase, util::display_command(command));
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().with_context(|| {
        format!(
            "[E_PROCESS] {phase}: cannot start {}",
            util::display_command(command)
        )
    })?;
    #[cfg(windows)]
    let job = match Job::attach(&child) {
        Ok(job) => job,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let stdout = child.stdout.take().context("missing stdout")?;
    let stderr = child.stderr.take().context("missing stderr")?;
    let read = |mut pipe: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut data = Vec::new();
            pipe.read_to_end(&mut data).map(|_| data)
        })
    };
    let out_thread = read(Box::new(stdout));
    let err_thread = read(Box::new(stderr));
    let started = Instant::now();
    let mut stop = None;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if util::CANCELLED.load(Ordering::Relaxed) {
            stop = Some("E_CANCELLED");
        } else if started.elapsed().as_secs() >= timeout {
            stop = Some("E_TIMEOUT");
        }
        if stop.is_some() {
            #[cfg(windows)]
            job.terminate();
            let _ = child.kill();
            break child.wait()?;
        }
        thread::sleep(Duration::from_millis(40));
    };
    #[cfg(windows)]
    drop(job);
    let stdout = String::from_utf8_lossy(
        &out_thread
            .join()
            .map_err(|_| anyhow::anyhow!("stdout reader panicked"))??,
    )
    .into_owned();
    let stderr = String::from_utf8_lossy(
        &err_thread
            .join()
            .map_err(|_| anyhow::anyhow!("stderr reader panicked"))??,
    )
    .into_owned();
    if !stdout.trim().is_empty() {
        report.detail(phase, &stdout);
    }
    if !stderr.trim().is_empty() {
        report.detail(phase, &stderr);
    }
    if let Some(reason) = stop {
        bail!(
            "[{reason}] {phase}: execution stopped after {:.1}s\n{}",
            started.elapsed().as_secs_f64(),
            stderr.trim()
        );
    }
    if !status.success() {
        bail!(
            "[E_PROCESS] {phase}: exit={}\n{}\n{}",
            status.code().unwrap_or(-1),
            stderr.trim(),
            stdout.trim()
        );
    }
    Ok(Captured { stdout, stderr })
}

#[cfg(windows)]
struct Job(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl Job {
    fn attach(child: &std::process::Child) -> Result<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::*;
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }
            let job = Self(handle);
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of_val(&info) as u32,
            ) == 0
                || AssignProcessToJobObject(handle, child.as_raw_handle()) == 0
            {
                return Err(std::io::Error::last_os_error())
                    .context("[E_PROCESS] cannot isolate child process tree");
            }
            Ok(job)
        }
    }
    fn terminate(&self) {
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.0, 1);
        }
    }
}
#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
