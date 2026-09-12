use nmlt_runtime::process::{
    FILE_BYTES, FailureKind, FileProcess, PIPE_BYTES, PROJECT_FILE_BYTES, Process,
};
use std::io::{Read, Write};
use std::process::Command;
use std::time::{Duration, Instant};

fn probe(mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "process_probe", "--nocapture"])
        .env("NMLT_ASYNC_PROBE", mode);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
}
#[test]
// These fixtures deliberately leave descendants for the supervisor to reap.
#[allow(clippy::zombie_processes)]
fn process_probe() {
    let Ok(mode) = std::env::var("NMLT_ASYNC_PROBE") else {
        return;
    };
    match mode.as_str() {
        "nested-project" => {
            for _ in 0..12 {
                let mut child = Process::start_project(
                    probe("input"),
                    b"hello".to_vec(),
                    Duration::from_secs(60),
                )
                .unwrap();
                let output = child.wait().as_ref().unwrap();
                assert_eq!(output.exit_code, Some(0));
                assert!(String::from_utf8_lossy(&output.stdout).contains("received"));
            }
            print!("nested-project-complete");
        }
        "file-bytes" => {
            let count: usize = std::env::var("NMLT_FILE_COUNT").unwrap().parse().unwrap();
            assert!(count <= PROJECT_FILE_BYTES as usize + PIPE_BYTES);
            let pattern = if std::env::var_os("NMLT_FILE_NO_LINES").is_some() {
                b"x".as_slice()
            } else {
                b"\x00\xff\r\nraw\rbytes".as_slice()
            };
            let block = (0..8192)
                .map(|n| pattern[n % pattern.len()])
                .collect::<Vec<_>>();
            let mut stdout = std::io::stdout().lock();
            let mut sent = 0;
            while sent < count {
                let size = (count - sent).min(block.len());
                stdout.write_all(&block[..size]).unwrap();
                sent += size;
            }
            stdout.flush().unwrap();
            // Omit the test harness footer so the emitted bytes are deterministic.
            std::process::exit(0);
        }
        "sleep" => std::thread::sleep(Duration::from_secs(5)),
        "stdout" => {
            let _ = std::io::stdout().write_all(&vec![b'x'; PIPE_BYTES * 2]);
        }
        "stderr" => {
            let _ = std::io::stderr().write_all(&vec![b'x'; PIPE_BYTES * 2]);
        }
        "exit" => {
            eprint!("error: rejected");
            std::process::exit(1);
        }
        "input" => {
            let mut input = vec![];
            std::io::stdin().read_to_end(&mut input).unwrap();
            assert_eq!(input, b"hello");
            print!("received");
        }
        "late-marker" => {
            std::thread::sleep(Duration::from_millis(800));
            std::fs::write(
                std::env::var_os("NMLT_PROCESS_MARKER").unwrap(),
                b"survived",
            )
            .unwrap();
        }
        "descendant" | "orphan" => {
            let marker = std::path::PathBuf::from(std::env::var_os("NMLT_PROCESS_MARKER").unwrap());
            let _child = probe("late-marker")
                .env("NMLT_PROCESS_MARKER", &marker)
                .spawn()
                .unwrap();
            std::fs::write(marker.with_extension("ready"), b"ready").unwrap();
            if mode == "orphan" {
                std::process::exit(0);
            }
            std::thread::sleep(Duration::from_secs(5));
        }
        #[cfg(windows)]
        "supervisor" => {
            let marker = std::path::PathBuf::from(std::env::var_os("NMLT_PROCESS_MARKER").unwrap());
            let mut command = probe("late-marker");
            command.env("NMLT_PROCESS_MARKER", &marker);
            let _child = Process::start(command, vec![], Duration::from_secs(5)).unwrap();
            std::fs::write(marker.with_extension("ready"), b"ready").unwrap();
            std::thread::sleep(Duration::from_secs(5));
        }
        #[cfg(windows)]
        "memory-limit" => {
            let mut data = Vec::<u8>::new();
            assert!(data.try_reserve_exact(2 * 1024 * 1024 * 1024).is_err());
            print!("memory-ceiling-enforced");
        }
        #[cfg(windows)]
        "process-limit" => {
            let mut children = vec![];
            for _ in 0..16 {
                match probe("sleep")
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    Ok(child) => children.push(child),
                    Err(_) => break,
                }
            }
            let count = children.len();
            for child in &mut children {
                let _ = child.kill();
                let _ = child.wait();
            }
            assert_eq!(
                count, 15,
                "root plus children must fit sixteen process slots"
            );
            print!("process-ceiling-enforced");
        }
        _ => panic!("unknown mode"),
    }
}

fn file_probe(count: usize) -> Command {
    let mut command = probe("file-bytes");
    command.env("NMLT_FILE_COUNT", count.to_string());
    command
}

#[test]
fn project_profile_is_explicit_and_does_not_validate_as_an_ordinary_job() {
    assert!(Process::start(probe("input"), b"hello".to_vec(), Duration::from_secs(60)).is_err());
    let mut child =
        Process::start_project(probe("input"), b"hello".to_vec(), Duration::from_secs(60)).unwrap();
    child.policy().validate_project().unwrap();
    assert!(child.policy().validate().is_err());
    assert_eq!(child.policy().memory_bytes, 8 * 1024 * 1024 * 1024);
    let mut changed = child.policy().clone();
    changed.memory_bytes -= 1;
    assert!(changed.validate_project().is_err());
    let output = child.wait().as_ref().unwrap();
    assert_eq!(output.exit_code, Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("received"));
    assert!(Process::start_project(probe("input"), vec![], Duration::from_secs(1801)).is_err());
}

#[test]
fn project_workers_can_supervise_sequential_nested_project_processes() {
    let mut child =
        Process::start_project_worker(probe("nested-project"), vec![], Duration::from_secs(60))
            .unwrap();
    child.policy().validate_project_worker().unwrap();
    assert!(child.policy().validate_project().is_err());
    assert!(child.policy().validate().is_err());
    let output = child.wait().as_ref().unwrap();
    assert_eq!(
        output.exit_code,
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("nested-project-complete"));
}

#[test]
fn project_file_capture_uses_its_own_bounded_receipt_and_policy() {
    let directory = std::env::temp_dir().join(format!(
        "nmlt-project-file-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let path = directory.join("larger.bin");
    let mut child = FileProcess::start_project(
        file_probe(FILE_BYTES as usize + 2048),
        vec![],
        Duration::from_secs(60),
        &path,
    )
    .unwrap();
    child.policy().validate_project().unwrap();
    assert!(child.policy().validate().is_err());
    let mut changed = child.policy().clone();
    changed.max_stdout_bytes += 1;
    assert!(changed.validate_project().is_err());
    let output = child.wait().as_ref().unwrap();
    assert_eq!(output.output.exit_code, Some(0));
    assert!(output.file.bytes > FILE_BYTES);
    assert!(output.file.validate().is_err());
    output.file.validate_project().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes.len() as u64, output.file.bytes);
    assert_eq!(nmlt_runtime::sha256(&bytes), output.file.sha256);
    let mut excessive = FileProcess::start_project(
        file_probe(PROJECT_FILE_BYTES as usize + 1),
        vec![],
        Duration::from_secs(60),
        &directory.join("excess.bin"),
    )
    .unwrap();
    let failure = excessive.wait().as_ref().unwrap_err();
    assert_eq!(failure.kind, FailureKind::OutputLimit);
    assert!(failure.child_reaped);
    assert!(
        std::fs::metadata(directory.join("excess.bin"))
            .unwrap()
            .len()
            <= PROJECT_FILE_BYTES
    );
}

#[test]
fn file_capture_preserves_raw_bytes_and_keeps_the_regular_policy_distinct() {
    let prefix = Process::start(file_probe(0), vec![], Duration::from_secs(5))
        .unwrap()
        .wait()
        .as_ref()
        .unwrap()
        .stdout
        .clone();
    let path = marker("raw-file");
    let count = 2 * PIPE_BYTES + 19;
    let mut child =
        FileProcess::start(file_probe(count), vec![], Duration::from_secs(5), &path).unwrap();
    child.policy().validate().unwrap();
    assert!(child.policy().process.validate().is_err());
    let mut altered_policy = child.policy().clone();
    altered_policy.max_stdout_bytes += 1;
    assert!(altered_policy.validate().is_err());
    let result = child.wait().as_ref().unwrap();
    assert_eq!(result.output.exit_code, Some(0));
    assert!(result.output.stdout.is_empty());
    assert!(result.output.stderr.is_empty());
    let bytes = std::fs::read(&path).unwrap();
    let pattern = b"\x00\xff\r\nraw\rbytes";
    let mut expected = prefix;
    expected.extend((0..count).map(|n| pattern[(n % 8192) % pattern.len()]));
    assert_eq!(bytes, expected);
    assert_eq!(result.file.bytes, bytes.len() as u64);
    assert_eq!(result.file.sha256, nmlt_runtime::sha256(&bytes));
    result.file.validate().unwrap();
}

#[test]
fn file_capture_enforces_exact_byte_ceiling_and_stderr_bound() {
    let prefix_len = Process::start(file_probe(0), vec![], Duration::from_secs(5))
        .unwrap()
        .wait()
        .as_ref()
        .unwrap()
        .stdout
        .len();
    let exact = marker("file-exact");
    let mut child = FileProcess::start(
        file_probe(FILE_BYTES as usize - prefix_len),
        vec![],
        Duration::from_secs(10),
        &exact,
    )
    .unwrap();
    assert_eq!(child.wait().as_ref().unwrap().file.bytes, FILE_BYTES);
    assert_eq!(std::fs::metadata(exact).unwrap().len(), FILE_BYTES);

    let over = marker("file-over");
    let mut child = FileProcess::start(
        file_probe(FILE_BYTES as usize - prefix_len + 1),
        vec![],
        Duration::from_secs(10),
        &over,
    )
    .unwrap();
    let failure = child.wait().as_ref().unwrap_err();
    assert_eq!(failure.kind, FailureKind::OutputLimit);
    assert!(failure.child_reaped);
    assert!(std::fs::metadata(over).unwrap().len() <= FILE_BYTES);

    let stderr = marker("file-stderr");
    let mut child =
        FileProcess::start(probe("stderr"), vec![], Duration::from_secs(5), &stderr).unwrap();
    assert_eq!(
        child.wait().as_ref().unwrap_err().kind,
        FailureKind::OutputLimit
    );
}

#[test]
fn file_capture_preserves_exit_status_and_does_not_overwrite_destinations() {
    let path = marker("existing-file");
    std::fs::write(&path, b"preserve").unwrap();
    assert!(FileProcess::start(probe("exit"), vec![], Duration::from_secs(5), &path).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"preserve");
    let path = marker("rejected-file");
    let mut child =
        FileProcess::start(probe("exit"), vec![], Duration::from_secs(5), &path).unwrap();
    let output = child.wait().as_ref().unwrap();
    assert_eq!(output.output.exit_code, Some(1));
    assert!(String::from_utf8_lossy(&output.output.stderr).contains("rejected"));
    assert_eq!(
        output.file.sha256,
        nmlt_runtime::sha256(&std::fs::read(path).unwrap())
    );
}

#[test]
fn file_capture_preserves_a_line_larger_than_the_decoded_buffer() {
    let prefix = Process::start(file_probe(0), vec![], Duration::from_secs(5))
        .unwrap()
        .wait()
        .as_ref()
        .unwrap()
        .stdout
        .clone();
    let path = marker("newline-free-file");
    let count = 3 * PIPE_BYTES + 7;
    let mut command = file_probe(count);
    command.env("NMLT_FILE_NO_LINES", "1");
    let mut child = FileProcess::start(command, vec![], Duration::from_secs(5), &path).unwrap();
    let result = child.wait().as_ref().unwrap();
    let mut expected = prefix;
    expected.extend(vec![b'x'; count]);
    assert_eq!(std::fs::read(path).unwrap(), expected);
    assert_eq!(result.file.sha256, nmlt_runtime::sha256(&expected));
    assert_eq!(result.file.bytes, expected.len() as u64);
}

#[test]
fn file_capture_cancellation_and_root_exit_reap_descendants() {
    for action in ["timeout", "cancel", "drop", "root-exit"] {
        let marker = marker(&format!("file-{action}"));
        let path = marker.with_extension("stdout");
        let mut command = probe(if action == "root-exit" {
            "orphan"
        } else {
            "descendant"
        });
        command.env("NMLT_PROCESS_MARKER", &marker);
        let timeout = if action == "timeout" {
            Duration::from_millis(200)
        } else {
            Duration::from_secs(5)
        };
        let mut child = FileProcess::start(command, vec![], timeout, &path).unwrap();
        await_ready(&marker);
        match action {
            "cancel" => {
                assert!(child.cancel().is_err());
            }
            "drop" => drop(child),
            _ => {
                assert!(child.wait().is_err());
            }
        }
        std::thread::sleep(Duration::from_millis(1000));
        assert!(!marker.exists(), "{action} left a descendant alive");
    }
}

#[test]
fn project_profiles_cancel_and_reap_the_complete_child_tree() {
    for kind in ["process", "file", "worker"] {
        let marker = marker(&format!("project-{kind}-cancel"));
        let mut command = probe("descendant");
        command.env("NMLT_PROCESS_MARKER", &marker);
        let failure = if kind == "file" {
            let path = marker.with_extension("stdout");
            let mut child =
                FileProcess::start_project(command, vec![], Duration::from_secs(60), &path)
                    .unwrap();
            child.policy().validate_project().unwrap();
            await_ready(&marker);
            child.cancel().as_ref().unwrap_err().clone()
        } else {
            let mut child = if kind == "worker" {
                Process::start_project_worker(command, vec![], Duration::from_secs(60)).unwrap()
            } else {
                Process::start_project(command, vec![], Duration::from_secs(60)).unwrap()
            };
            if kind == "worker" {
                child.policy().validate_project_worker().unwrap();
            } else {
                child.policy().validate_project().unwrap();
            }
            await_ready(&marker);
            child.cancel().as_ref().unwrap_err().clone()
        };
        assert!(failure.child_reaped);
        assert_eq!(failure.kind, FailureKind::Cancelled);
        std::thread::sleep(Duration::from_millis(1000));
        assert!(!marker.exists(), "project profile left a descendant alive");
    }
}

fn marker(name: &str) -> std::path::PathBuf {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/process-tree-tests");
    std::fs::create_dir_all(&root).unwrap();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    root.join(format!("{name}-{}-{stamp}", std::process::id()))
}
fn await_ready(marker: &std::path::Path) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !marker.with_extension("ready").exists() {
        assert!(Instant::now() < deadline, "descendant was not started");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn timeout_cancel_drop_and_root_exit_do_not_leave_descendants() {
    for mode in ["timeout", "cancel", "drop", "orphan"] {
        let marker = marker(mode);
        let mut command = probe(if mode == "orphan" {
            "orphan"
        } else {
            "descendant"
        });
        command.env("NMLT_PROCESS_MARKER", &marker);
        let mut child = Process::start(
            command,
            vec![],
            if mode == "timeout" {
                Duration::from_millis(400)
            } else {
                Duration::from_secs(5)
            },
        )
        .unwrap();
        child.policy().validate().unwrap();
        await_ready(&marker);
        if mode == "cancel" {
            assert!(child.cancel().as_ref().unwrap_err().child_reaped);
        } else if mode != "drop" {
            assert!(child.wait().as_ref().unwrap_err().child_reaped);
        }
        drop(child);
        std::thread::sleep(Duration::from_millis(950));
        assert!(!marker.exists(), "{mode} left a descendant alive");
    }
}

#[cfg(windows)]
#[test]
fn windows_job_limits_and_abrupt_supervisor_death_are_enforced() {
    for mode in ["memory-limit", "process-limit"] {
        let mut child = Process::start(probe(mode), vec![], Duration::from_secs(5)).unwrap();
        assert_eq!(child.policy().mechanism, "job_object");
        assert_eq!(child.policy().max_processes, Some(16));
        let output = child
            .wait()
            .as_ref()
            .unwrap_or_else(|error| panic!("{mode}: {error:?}"));
        assert_eq!(
            output.exit_code,
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let marker = marker("parent-death");
    let mut supervisor = probe("supervisor")
        .env("NMLT_PROCESS_MARKER", &marker)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    await_ready(&marker);
    supervisor.kill().unwrap();
    supervisor.wait().unwrap();
    std::thread::sleep(Duration::from_millis(1100));
    assert!(
        !marker.exists(),
        "kernel job must close when supervisor is killed"
    );
}
#[test]
fn completed_child_does_not_timeout_when_polled_late() {
    let mut child =
        Process::start(probe("input"), b"hello".to_vec(), Duration::from_secs(1)).unwrap();
    std::thread::sleep(Duration::from_millis(1300));
    let output = child.poll().unwrap().as_ref().unwrap();
    assert_eq!(output.exit_code, Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("received"));
}
#[test]
fn deadline_fires_without_polling_and_blocked_stdin_is_reaped() {
    let mut child = Process::start(
        probe("sleep"),
        vec![0; PIPE_BYTES],
        Duration::from_millis(200),
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(400));
    let error = child.wait().as_ref().unwrap_err();
    assert_eq!(error.kind, FailureKind::Timeout);
    assert!(error.child_reaped);
}
#[test]
fn cancellation_is_prompt_and_reports_direct_child_cleanup() {
    let mut child = Process::start(probe("sleep"), vec![], Duration::from_secs(5)).unwrap();
    let start = Instant::now();
    let error = child.cancel().as_ref().unwrap_err();
    assert_eq!(error.kind, FailureKind::Cancelled);
    assert!(error.child_reaped);
    assert!(start.elapsed() < Duration::from_secs(2));
    assert_eq!(
        child.wait().as_ref().unwrap_err().kind,
        FailureKind::Cancelled
    );
}
#[test]
fn both_pipes_are_bounded_while_checker_rejections_remain_observable() {
    for mode in ["stdout", "stderr"] {
        let mut child = Process::start(probe(mode), vec![], Duration::from_secs(5)).unwrap();
        let error = child.wait().as_ref().unwrap_err();
        assert_eq!(error.kind, FailureKind::OutputLimit);
        assert!(error.child_reaped);
    }
    let mut child = Process::start(probe("exit"), vec![], Duration::from_secs(5)).unwrap();
    let output = child.wait().as_ref().unwrap();
    assert_eq!(output.exit_code, Some(1));
    assert_eq!(output.stderr, b"error: rejected");
}
#[test]
fn invalid_bounds_fail_before_spawn() {
    for duration in [Duration::ZERO, Duration::from_secs(31)] {
        assert!(Process::start(probe("sleep"), vec![], duration).is_err());
    }
    assert!(
        Process::start(
            probe("sleep"),
            vec![0; PIPE_BYTES + 1],
            Duration::from_secs(1)
        )
        .is_err()
    );
}
