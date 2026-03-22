use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::raw::c_int;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Utc;

use crate::config::Config;
use crate::dashboard::{
    initial_snapshot, log_supervisor_message, now_rfc3339, record_event, record_log,
    spawn_dashboard_server, update_service, ExitInfo, SharedDashboardState,
};
use crate::logging::write_aggregated_log_line;
use crate::services::{ServiceCommand, ServiceSpec, SERVICES};

const POLL_INTERVAL: Duration = Duration::from_millis(250);
const BASE_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(5);
const FAST_FAILURE_WINDOW: Duration = Duration::from_secs(3);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

const SIGINT: c_int = 2;
const SIGTERM: c_int = 15;
const SIGKILL: c_int = 9;

static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
struct ManagedService {
    spec: ServiceSpec,
    child: Option<Child>,
    started_at: Option<Instant>,
    restart_count: u32,
    next_restart_at: Option<Instant>,
    ever_started: bool,
    completed_successfully: bool,
}

unsafe extern "C" {
    fn kill(pid: c_int, sig: c_int) -> c_int;
    fn setpgid(pid: c_int, pgid: c_int) -> c_int;
    fn signal(sig: c_int, handler: usize) -> usize;
}

extern "C" fn handle_shutdown_signal(_: c_int) {
    SHUTTING_DOWN.store(true, Ordering::SeqCst);
}

fn install_signal_handlers() {
    let handler = handle_shutdown_signal as *const () as usize;
    unsafe {
        signal(SIGINT, handler);
        signal(SIGTERM, handler);
    }
}

fn spawn_service(
    spec: ServiceSpec,
    config: &Config,
    restart_count: u32,
    dashboard_state: &SharedDashboardState,
) -> io::Result<ManagedService> {
    let shell_command = spec.shell_command(config);
    let mut command = Command::new("/usr/bin/env");
    command.arg("bash").arg("-c").arg(&shell_command);
    if let Some(cwd) = spec.cwd {
        command.current_dir(config.resolve_path(cwd));
    }
    command.envs((spec.env)(config));
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    unsafe {
        command.pre_exec(|| {
            let result = setpgid(0, 0);
            if result == 0 {
                return Ok(());
            }
            Err(io::Error::last_os_error())
        });
    }

    let mut child = command.spawn()?;
    if let Some(stdout) = child.stdout.take() {
        stream_output(
            spec.name,
            "stdout",
            stdout,
            false,
            Some(dashboard_state.clone()),
        );
    }
    if let Some(stderr) = child.stderr.take() {
        stream_output(
            spec.name,
            "stderr",
            stderr,
            true,
            Some(dashboard_state.clone()),
        );
    }
    log_supervisor_event(&format!(
        "service={} event=spawn pid={} restarts={}",
        spec.name,
        child.id(),
        restart_count
    ));

    Ok(ManagedService {
        spec,
        child: Some(child),
        started_at: Some(Instant::now()),
        restart_count,
        next_restart_at: None,
        ever_started: true,
        completed_successfully: false,
    })
}

fn timestamp_prefix() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn log_supervisor_event(message: &str) {
    log_supervisor_message(message);
}

fn update_running_state(state: &SharedDashboardState, service: &ManagedService) {
    update_service(state, service.spec.name, |snapshot| {
        snapshot.state = String::from("running");
        snapshot.pid = service.child.as_ref().map(Child::id);
        snapshot.restart_count = service.restart_count;
        snapshot.started_at = Some(now_rfc3339());
        snapshot.next_restart_at = None;
        snapshot.message = String::from("Running");
    });
}

fn update_completed_state(
    state: &SharedDashboardState,
    service_name: &str,
    runtime: Duration,
    exit_info: ExitInfo,
) {
    update_service(state, service_name, |snapshot| {
        snapshot.state = String::from("completed");
        snapshot.pid = None;
        snapshot.next_restart_at = None;
        snapshot.last_exit = Some(exit_info);
        snapshot.message = format!("Completed in {} ms", runtime.as_millis());
    });
}

fn update_scheduled_restart_state(
    state: &SharedDashboardState,
    service_name: &str,
    restart_count: u32,
    delay: Duration,
    message: String,
) {
    let next_restart_at = Utc::now() + chrono::Duration::from_std(delay).unwrap_or_default();
    update_service(state, service_name, |snapshot| {
        snapshot.state = String::from("restarting");
        snapshot.pid = None;
        snapshot.restart_count = restart_count;
        snapshot.next_restart_at = Some(next_restart_at.format("%Y-%m-%dT%H:%M:%SZ").to_string());
        snapshot.message = message;
    });
}

fn format_log_line(service_name: &str, buffer: &[u8]) -> Vec<u8> {
    let mut line = format!("{} [{}] ", timestamp_prefix(), service_name).into_bytes();
    line.extend_from_slice(buffer);
    if !buffer.ends_with(b"\n") {
        line.push(b'\n');
    }
    line
}

fn log_stream_line(output: &mut impl Write, service_name: &str, buffer: &[u8]) -> io::Result<()> {
    let line = format_log_line(service_name, buffer);
    output.write_all(&line)?;
    output.flush()
}

fn stream_output<R>(
    service_name: &'static str,
    stream_name: &'static str,
    reader: R,
    stderr: bool,
    dashboard_state: Option<SharedDashboardState>,
) where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut buffer = Vec::new();

        loop {
            buffer.clear();
            match reader.read_until(b'\n', &mut buffer) {
                Ok(0) => break,
                Ok(_) => {
                    if let Some(state) = &dashboard_state {
                        let line = String::from_utf8_lossy(&buffer);
                        record_log(state, service_name, stream_name, &line);
                    }
                    let write_result = if stderr {
                        let mut output = io::stderr().lock();
                        log_stream_line(&mut output, service_name, &buffer)
                    } else {
                        let mut output = io::stdout().lock();
                        log_stream_line(&mut output, service_name, &buffer)
                    };

                    if let Err(error) = write_result {
                        log_supervisor_event(&format!(
                            "service={} stream={} event=log-write-failed error={}",
                            service_name, stream_name, error
                        ));
                        break;
                    }

                    if let Err(error) =
                        write_aggregated_log_line(&format_log_line(service_name, &buffer))
                    {
                        log_supervisor_event(&format!(
                            "service={} stream={} event=file-log-write-failed error={}",
                            service_name, stream_name, error
                        ));
                        break;
                    }
                }
                Err(error) => {
                    log_supervisor_event(&format!(
                        "service={} stream={} event=log-read-failed error={}",
                        service_name, stream_name, error
                    ));
                    break;
                }
            }
        }
    });
}

fn schedule_restart(service: &mut ManagedService, delay: Duration) {
    service.child = None;
    service.started_at = None;
    service.next_restart_at = Some(Instant::now() + delay);
    service.completed_successfully = false;
}

fn dependencies_ready(
    services: &HashMap<&'static str, ManagedService>,
    service: &ManagedService,
) -> bool {
    service.spec.depends_on.iter().all(|dependency_name| {
        services
            .get(dependency_name)
            .map(|dependency| {
                if dependency.spec.is_setup_only() {
                    dependency.completed_successfully
                } else {
                    dependency.child.is_some()
                }
            })
            .unwrap_or(false)
    })
}

fn update_waiting_for_dependencies_state(
    state: &SharedDashboardState,
    service_name: &str,
    dependencies: &[&str],
) {
    if dependencies.is_empty() {
        return;
    }
    update_service(state, service_name, |snapshot| {
        snapshot.state = String::from("waiting");
        snapshot.pid = None;
        snapshot.next_restart_at = None;
        snapshot.message = format!("Waiting for {}", dependencies.join(", "));
    });
}

fn spawn_or_schedule(
    service: &mut ManagedService,
    config: &Config,
    dashboard_state: &SharedDashboardState,
    restart_count: u32,
) {
    match spawn_service(service.spec, config, restart_count, dashboard_state) {
        Ok(restarted_service) => {
            update_running_state(dashboard_state, &restarted_service);
            record_event(
                dashboard_state,
                format!(
                    "service={} event={} restart_count={restart_count}",
                    service.spec.name,
                    if service.ever_started {
                        "restarted"
                    } else {
                        "spawned"
                    }
                ),
            );
            *service = restarted_service;
        }
        Err(error) => {
            let backoff = restart_backoff(Duration::ZERO, restart_count);
            service.restart_count = restart_count;
            service.next_restart_at = Some(Instant::now() + backoff);
            service.ever_started = true;
            service.completed_successfully = false;
            update_scheduled_restart_state(
                dashboard_state,
                service.spec.name,
                restart_count,
                backoff,
                if restart_count == 0 {
                    format!("Spawn failed: {error}")
                } else {
                    format!("Restart failed: {error}")
                },
            );
            log_supervisor_event(&format!(
                "service={} event={} error={} restart_in_ms={}",
                service.spec.name,
                if restart_count == 0 {
                    "spawn-failed"
                } else {
                    "restart-failed"
                },
                error,
                backoff.as_millis()
            ));
            record_event(
                dashboard_state,
                format!(
                    "service={} event={} error={}",
                    service.spec.name,
                    if restart_count == 0 {
                        "spawn-failed"
                    } else {
                        "restart-failed"
                    },
                    error
                ),
            );
        }
    }
}

fn describe_exit(status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("code={code}"),
        None => String::from("signal=unknown"),
    }
}

fn restart_backoff(runtime: Duration, restart_count: u32) -> Duration {
    if runtime >= FAST_FAILURE_WINDOW {
        return BASE_BACKOFF;
    }

    match restart_count {
        0 | 1 => BASE_BACKOFF,
        2 => Duration::from_secs(2),
        3 => Duration::from_secs(3),
        _ => MAX_BACKOFF,
    }
}

fn signal_process_group(pid: u32, signal_number: c_int) {
    let pgid = -(pid as i32);
    let result = unsafe { kill(pgid, signal_number) };
    if result != 0 {
        let error = io::Error::last_os_error();
        log_supervisor_event(&format!(
            "event=signal-failed target_pgid={} signal={} error={}",
            -pgid, signal_number, error
        ));
    }
}

fn running_service_pids(services: &HashMap<&'static str, ManagedService>) -> Vec<u32> {
    services
        .values()
        .filter_map(|service| service.child.as_ref().map(Child::id))
        .collect()
}

fn begin_shutdown(services: &mut HashMap<&'static str, ManagedService>) {
    let running_pids = running_service_pids(services);

    for service in services.values_mut() {
        service.next_restart_at = None;
    }

    for pid in running_pids {
        signal_process_group(pid, SIGTERM);
    }
}

fn finish_shutdown(services: &mut HashMap<&'static str, ManagedService>) {
    let deadline = Instant::now() + SHUTDOWN_TIMEOUT;

    loop {
        let mut running = Vec::new();

        for service in services.values_mut() {
            let Some(child) = service.child.as_mut() else {
                continue;
            };

            match child.try_wait() {
                Ok(Some(status)) => {
                    service.child = None;
                    service.started_at = None;
                    log_supervisor_event(&format!(
                        "service={} event=shutdown-exit {}",
                        service.spec.name,
                        describe_exit(status)
                    ));
                }
                Ok(None) => running.push(service.spec.name),
                Err(error) => {
                    log_supervisor_event(&format!(
                        "service={} event=shutdown-poll-failed error={}",
                        service.spec.name, error
                    ));
                }
            }
        }

        if running.is_empty() {
            return;
        }

        if Instant::now() >= deadline {
            for service_name in &running {
                if let Some(service) = services.get_mut(service_name) {
                    if let Some(child) = service.child.as_ref() {
                        signal_process_group(child.id(), SIGKILL);
                    }
                }
            }

            for service_name in &running {
                if let Some(service) = services.get_mut(service_name) {
                    if let Some(child) = service.child.as_mut() {
                        let _ = child.wait();
                    }
                }
            }
            return;
        }

        thread::sleep(POLL_INTERVAL);
    }
}

pub fn run_supervisor(config: &Config) -> Result<(), String> {
    install_signal_handlers();
    let dashboard_state = Arc::new(Mutex::new(initial_snapshot()));
    spawn_dashboard_server(config, dashboard_state.clone())?;

    let mut services = HashMap::new();

    for spec in SERVICES {
        services.insert(
            spec.name,
            ManagedService {
                spec: *spec,
                child: None,
                started_at: None,
                restart_count: 0,
                next_restart_at: None,
                ever_started: false,
                completed_successfully: false,
            },
        );
        update_waiting_for_dependencies_state(&dashboard_state, spec.name, spec.depends_on);
    }

    loop {
        if SHUTTING_DOWN.load(Ordering::SeqCst) {
            log_supervisor_event("event=shutdown-start");
            record_event(&dashboard_state, String::from("event=shutdown-start"));
            for spec in SERVICES {
                update_service(&dashboard_state, spec.name, |snapshot| {
                    snapshot.state = String::from("stopping");
                    snapshot.pid = None;
                    snapshot.next_restart_at = None;
                    snapshot.message = String::from("Stopping");
                });
            }
            begin_shutdown(&mut services);
            finish_shutdown(&mut services);
            log_supervisor_event("event=shutdown-complete");
            record_event(&dashboard_state, String::from("event=shutdown-complete"));
            return Ok(());
        }

        for spec in SERVICES {
            let deps_ready = services
                .get(spec.name)
                .map(|service| dependencies_ready(&services, service))
                .unwrap_or(false);
            let Some(service) = services.get_mut(spec.name) else {
                log_supervisor_event(&format!("service={} event=state-missing", spec.name));
                continue;
            };

            if !deps_ready {
                if service.child.is_none() && service.next_restart_at.is_none() {
                    update_waiting_for_dependencies_state(
                        &dashboard_state,
                        service.spec.name,
                        service.spec.depends_on,
                    );
                }
                continue;
            }

            if !service.ever_started && service.child.is_none() && service.next_restart_at.is_none()
            {
                spawn_or_schedule(service, config, &dashboard_state, 0);
                continue;
            }

            if let Some(next_restart_at) = service.next_restart_at {
                if Instant::now() >= next_restart_at {
                    let restart_count = if service.ever_started {
                        service.restart_count + 1
                    } else {
                        0
                    };
                    spawn_or_schedule(service, config, &dashboard_state, restart_count);
                }
                continue;
            }

            let Some(child) = service.child.as_mut() else {
                if service.spec.is_setup_only() {
                    continue;
                }
                schedule_restart(service, BASE_BACKOFF);
                update_scheduled_restart_state(
                    &dashboard_state,
                    service.spec.name,
                    service.restart_count,
                    BASE_BACKOFF,
                    String::from("Child process missing"),
                );
                log_supervisor_event(&format!(
                    "service={} event=child-missing restart_in_ms={}",
                    service.spec.name,
                    BASE_BACKOFF.as_millis()
                ));
                record_event(
                    &dashboard_state,
                    format!("service={} event=child-missing", service.spec.name),
                );
                continue;
            };

            match child.try_wait() {
                Ok(Some(status)) => {
                    let runtime = service
                        .started_at
                        .map(|started_at| started_at.elapsed())
                        .unwrap_or_default();
                    let backoff = restart_backoff(runtime, service.restart_count);
                    service.child = None;
                    service.started_at = None;
                    match service.spec.command {
                        ServiceCommand::SetupOnly => {
                            log_supervisor_event(&format!(
                                "service={} event=exit {} runtime_ms={}",
                                service.spec.name,
                                describe_exit(status),
                                runtime.as_millis()
                            ));
                        }
                        ServiceCommand::Run(_) | ServiceCommand::RunWithConfig(_) => {
                            log_supervisor_event(&format!(
                                "service={} event=exit {} runtime_ms={} restart_in_ms={}",
                                service.spec.name,
                                describe_exit(status),
                                runtime.as_millis(),
                                backoff.as_millis()
                            ));
                        }
                    }
                    let exit_info = ExitInfo {
                        detail: describe_exit(status),
                        at: now_rfc3339(),
                    };
                    if service.spec.is_setup_only() {
                        if status.success() {
                            service.completed_successfully = true;
                            update_completed_state(
                                &dashboard_state,
                                service.spec.name,
                                runtime,
                                exit_info,
                            );
                        } else {
                            update_scheduled_restart_state(
                                &dashboard_state,
                                service.spec.name,
                                service.restart_count,
                                backoff,
                                format!("Exited after {} ms; retrying", runtime.as_millis()),
                            );
                        }
                    } else {
                        update_service(&dashboard_state, service.spec.name, |snapshot| {
                            snapshot.state = if SHUTTING_DOWN.load(Ordering::SeqCst) {
                                String::from("stopped")
                            } else {
                                String::from("restarting")
                            };
                            snapshot.pid = None;
                            snapshot.last_exit = Some(exit_info);
                            snapshot.message = format!("Exited after {} ms", runtime.as_millis());
                            if SHUTTING_DOWN.load(Ordering::SeqCst) {
                                snapshot.next_restart_at = None;
                            } else {
                                let next_restart_at = Utc::now()
                                    + chrono::Duration::from_std(backoff).unwrap_or_default();
                                snapshot.next_restart_at =
                                    Some(next_restart_at.format("%Y-%m-%dT%H:%M:%SZ").to_string());
                            }
                        });
                    }
                    record_event(
                        &dashboard_state,
                        format!(
                            "service={} event=exit {}",
                            service.spec.name,
                            describe_exit(status)
                        ),
                    );
                    if SHUTTING_DOWN.load(Ordering::SeqCst)
                        || (service.spec.is_setup_only() && status.success())
                    {
                        service.next_restart_at = None;
                    } else {
                        service.next_restart_at = Some(Instant::now() + backoff);
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    schedule_restart(service, BASE_BACKOFF);
                    update_scheduled_restart_state(
                        &dashboard_state,
                        service.spec.name,
                        service.restart_count,
                        BASE_BACKOFF,
                        format!("Polling failed: {error}"),
                    );
                    log_supervisor_event(&format!(
                        "service={} event=poll-failed error={} restart_in_ms={}",
                        service.spec.name,
                        error,
                        BASE_BACKOFF.as_millis()
                    ));
                    record_event(
                        &dashboard_state,
                        format!(
                            "service={} event=poll-failed error={}",
                            service.spec.name, error
                        ),
                    );
                }
            }
        }

        thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_env(_: &Config) -> Vec<(&'static str, String)> {
        Vec::new()
    }

    const TEST_SPEC: ServiceSpec = ServiceSpec {
        name: "test",
        setup_command: None,
        command: ServiceCommand::Run("exec true"),
        cwd: None,
        env: empty_env,
        depends_on: &[],
        dashboard: crate::services::ServiceDashboard {
            title: "Test",
            description: "Test service",
            href: None,
            endpoints: &[],
            show: false,
        },
    };

    #[test]
    fn running_service_pids_excludes_restart_waiting_services() {
        let child = Command::new("/usr/bin/env")
            .arg("bash")
            .arg("-c")
            .arg("exec sleep 30")
            .spawn()
            .expect("spawn sleep");
        let running_pid = child.id();

        let mut services = HashMap::new();
        services.insert(
            "running",
            ManagedService {
                spec: TEST_SPEC,
                child: Some(child),
                started_at: Some(Instant::now()),
                restart_count: 0,
                next_restart_at: None,
                ever_started: true,
                completed_successfully: false,
            },
        );
        services.insert(
            "waiting",
            ManagedService {
                spec: TEST_SPEC,
                child: None,
                started_at: None,
                restart_count: 1,
                next_restart_at: Some(Instant::now() + BASE_BACKOFF),
                ever_started: true,
                completed_successfully: false,
            },
        );

        assert_eq!(running_service_pids(&services), vec![running_pid]);

        if let Some(service) = services.get_mut("running") {
            if let Some(child) = service.child.as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}
