use std::fs;
use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nmlt_runtime::*;

fn directory(label: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/job-journal-tests");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = root.join(format!("{label}-{}-{unique}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir.canonicalize().unwrap()
}
fn spec() -> RunSpec {
    RunSpec {
        run_id: "journal-test".into(),
        context_sha256: sha256(b"test context"),
        limits: Limits {
            slots: 1,
            generations_per_slot: 2,
            max_attempts: 2,
            max_events: 40,
            work_budget: 2,
        },
    }
}
fn reserve(journal: &mut Journal) -> Control {
    let Receipt::Control { control } = journal
        .apply(Command::Reserve {
            task: "square".into(),
            owner: "worker".into(),
            request: Request {
                adapter: worker::adapter(),
                context_sha256: spec().context_sha256,
                input: Value::Int(5),
                reserved_work: 1,
            },
        })
        .unwrap()
    else {
        panic!()
    };
    control
}
fn start(journal: &mut Journal) -> (Control, Dispatch) {
    let control = reserve(journal);
    let Receipt::Dispatch { dispatch, control } =
        journal.apply(Command::Dispatch { control }).unwrap()
    else {
        panic!()
    };
    (control, dispatch)
}

#[test]
fn restart_replays_every_committed_phase_without_redispatching() {
    for target in ["reserved", "running", "cancelling", "finished", "collected"] {
        let dir = directory(target);
        let path = dir.join("run.jsonl");
        let mut journal = Journal::create(&path, spec()).unwrap();
        let mut control = reserve(&mut journal);
        if target != "reserved" {
            let Receipt::Dispatch {
                dispatch,
                control: next,
            } = journal.apply(Command::Dispatch { control }).unwrap()
            else {
                panic!()
            };
            control = next;
            if target == "cancelling" {
                let Receipt::Control { control: next } =
                    journal.apply(Command::Cancel { control }).unwrap()
                else {
                    panic!()
                };
                control = next;
            } else if matches!(target, "finished" | "collected") {
                let Receipt::Control { control: next } = journal
                    .apply(Command::Deliver {
                        response: worker::evaluate(&dispatch).unwrap(),
                    })
                    .unwrap()
                else {
                    panic!()
                };
                control = next;
                if target == "collected" {
                    journal
                        .apply(Command::Collect {
                            control: control.clone(),
                        })
                        .unwrap();
                }
            }
        }
        let accounting = journal.state().accounting();
        drop(journal);
        let mut restored = Journal::open(&path, &spec()).unwrap();
        assert_eq!(restored.state().accounting(), accounting);
        assert!(restored.apply(Command::Dispatch { control }).is_err());
        assert!(match (&restored.state().attempts()[0].phase, target) {
            (Phase::Reserved, "reserved")
            | (Phase::Finished { .. }, "finished")
            | (Phase::Collected { .. }, "collected") => true,
            (
                Phase::Uncertain {
                    cancellation_requested,
                },
                "running",
            ) => !cancellation_requested,
            (
                Phase::Uncertain {
                    cancellation_requested,
                },
                "cancelling",
            ) => *cancellation_requested,
            _ => false,
        });
        if target == "finished" {
            let control = restored.state().attempts()[0].control.clone();
            let Receipt::Collected { outcome, .. } =
                restored.apply(Command::Collect { control }).unwrap()
            else {
                panic!()
            };
            assert_eq!(
                outcome,
                Outcome::Completed {
                    value: Value::Int(25)
                }
            );
        }
    }
}

#[test]
fn exclusive_access_rejects_other_handles_and_unsafe_aliases() {
    let dir = directory("lock");
    let path = dir.join("run.jsonl");
    let journal = Journal::create(&path, spec()).unwrap();
    assert!(Journal::open(&path, &spec()).is_err());
    let alias = dir.join("alias.jsonl");
    fs::hard_link(&path, &alias).unwrap();
    assert!(Journal::open(&alias, &spec()).is_err());
    drop(journal);
    #[cfg(unix)]
    {
        assert!(Journal::open(&alias, &spec()).is_err());
        fs::remove_file(&alias).unwrap();
        Journal::open(&path, &spec()).unwrap();
    }
    #[cfg(not(unix))]
    Journal::open(&alias, &spec()).unwrap();
}

#[cfg(unix)]
#[test]
fn adding_a_hardlink_stops_further_commits_on_an_existing_unix_handle() {
    let dir = directory("new-alias");
    let path = dir.join("run.jsonl");
    let mut journal = Journal::create(&path, spec()).unwrap();
    let control = reserve(&mut journal);
    fs::hard_link(&path, dir.join("alias.jsonl")).unwrap();
    assert!(journal.apply(Command::Dispatch { control }).is_err());
    assert_eq!(journal.state().accounting().dispatched_attempts, 0);
    assert!(
        journal
            .apply(Command::Recover)
            .unwrap_err()
            .0
            .contains("poisoned")
    );
}

#[test]
fn existing_files_and_rejected_operations_are_unchanged() {
    let dir = directory("unchanged");
    let path = dir.join("run.jsonl");
    let mut journal = Journal::create(&path, spec()).unwrap();
    let c = reserve(&mut journal);
    // Use the same locked handle's inspection state; compare bytes after drop.
    let expected = journal.state().clone();
    let mut wrong = c;
    wrong.owner = "other".into();
    assert!(journal.apply(Command::Dispatch { control: wrong }).is_err());
    assert_eq!(journal.state(), &expected);
    drop(journal);
    let before = fs::read(&path).unwrap();
    assert!(Journal::create(&path, spec()).is_err());
    let mut different = spec();
    different.context_sha256 = sha256(b"changed definition");
    assert!(Journal::open(&path, &different).is_err());
    different = spec();
    different.limits.work_budget += 1;
    assert!(Journal::open(&path, &different).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(before.iter().filter(|b| **b == b'\n').count(), 2);
}

#[test]
fn corrupted_reordered_and_torn_records_are_rejected_without_repairing_them() {
    let dir = directory("invalid");
    let path = dir.join("run.jsonl");
    let mut journal = Journal::create(&path, spec()).unwrap();
    start(&mut journal);
    drop(journal);
    let original = fs::read_to_string(&path).unwrap();
    let lines: Vec<_> = original.lines().collect();
    let tail_start = lines[0].len() + lines[1].len() + 2;
    let tail_len = original.len() - tail_start;
    let mut variants = vec![
        original.replacen("nmlt-job-journal-v1", "nmlt-job-journal-v2", 1),
        original.replacen(
            "\"implementation_sha256\":\"",
            "\"implementation_sha256\":\"0",
            1,
        ),
        original.replacen("\"sequence\":2", "\"sequence\":3", 1),
        original.replacen("\"previous_sha256\":\"", "\"previous_sha256\":\"0", 1),
        original.replacen("\"state_sha256\":\"", "\"state_sha256\":\"0", 1),
        original.replacen("\"revision\":0", "\"revision\":1", 1),
        original.replacen("{\"schema\"", "{\"extra\":true,\"schema\"", 1),
        format!("{}\n{}\n{}\n", lines[0], lines[2], lines[1]),
        format!("{original}{}\n", lines[2]),
    ];
    for cut in [1, tail_len / 2, tail_len - 1] {
        variants.push(original[..tail_start + cut].into());
    }
    for (i, changed) in variants.into_iter().enumerate() {
        assert_ne!(changed, original, "ineffective corruption {i}");
        let damaged = dir.join(format!("damaged-{i}.jsonl"));
        fs::write(&damaged, &changed).unwrap();
        assert!(
            Journal::open(&damaged, &spec()).is_err(),
            "accepted corruption {i}"
        );
        assert_eq!(fs::read_to_string(&damaged).unwrap(), changed);
    }
}

#[test]
fn cancellation_and_late_delivery_survive_reopen_and_slot_reuse() {
    let dir = directory("late");
    let path = dir.join("run.jsonl");
    let mut journal = Journal::create(&path, spec()).unwrap();
    let (control, first) = start(&mut journal);
    journal.apply(Command::Cancel { control }).unwrap();
    drop(journal);
    let mut journal = Journal::open(&path, &spec()).unwrap();
    let control = journal.state().attempts()[0].control.clone();
    let old_response = worker::evaluate(&first).unwrap();
    let Receipt::Control { control } = journal
        .apply(Command::Reconcile {
            control,
            response: old_response.clone(),
        })
        .unwrap()
    else {
        panic!()
    };
    journal.apply(Command::Collect { control }).unwrap();
    let (_, second) = start(&mut journal);
    assert_ne!(first.binding.attempt, second.binding.attempt);
    let current = journal.state().attempts()[1].clone();
    assert!(matches!(
        journal
            .apply(Command::Deliver {
                response: old_response
            })
            .unwrap(),
        Receipt::Ignored { .. }
    ));
    assert_eq!(journal.state().attempts()[1], current);
    let Receipt::Control { control } = journal
        .apply(Command::Deliver {
            response: worker::evaluate(&second).unwrap(),
        })
        .unwrap()
    else {
        panic!()
    };
    journal.apply(Command::Collect { control }).unwrap();
    let expected = journal.state().clone();
    drop(journal);
    let restored = Journal::open(&path, &spec()).unwrap();
    assert_eq!(restored.state(), &expected);
    assert_eq!(restored.state().accounting().charged_work, 2);
}

// Spawned by the test below to exercise an actual process death at the durable
// dispatch boundary or after worker output survives outside the journal.
// Normal test runs return immediately without extra work.
#[test]
fn journal_child() {
    let Some(path) = std::env::var_os("NMLT_JOB_CRASH_TEST_PATH") else {
        return;
    };
    let mut journal = Journal::create(Path::new(&path), spec()).unwrap();
    let (_, dispatch) = start(&mut journal);
    if std::env::var_os("NMLT_JOB_CRASH_AFTER_OUTPUT").is_some() {
        let response = worker::evaluate(&dispatch).unwrap();
        let mut output =
            fs::File::create_new(Path::new(&path).with_extension("output.json")).unwrap();
        output
            .write_all(&serde_json::to_vec(&response).unwrap())
            .unwrap();
        output.sync_all().unwrap();
        // Deliberately leave the journal without a completion record.
    }
    println!("NMLT_DISPATCH_COMMITTED");
    std::io::stdout().flush().unwrap();
    let mut barrier = [0];
    std::io::stdin().read_exact(&mut barrier).unwrap();
    panic!("parent should kill this process before releasing the barrier");
}

#[test]
fn process_kill_releases_lock_and_recovers_dispatch_as_uncertain() {
    for after_output in [false, true] {
        kill_and_recover(after_output);
    }
}

fn kill_and_recover(after_output: bool) {
    let path = directory("crash").join("run.jsonl");
    let mut process = Process::new(std::env::current_exe().unwrap());
    process
        .args(["--exact", "journal_child", "--nocapture"])
        .env("NMLT_JOB_CRASH_TEST_PATH", &path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if after_output {
        process.env("NMLT_JOB_CRASH_AFTER_OUTPUT", "1");
    } else {
        process.env_remove("NMLT_JOB_CRASH_AFTER_OUTPUT");
    }
    let mut child = process.spawn().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let reached = std::io::BufReader::new(stdout)
            .lines()
            .any(|line| line.is_ok_and(|line| line.contains("NMLT_DISPATCH_COMMITTED")));
        let _ = send.send(reached);
    });
    let reached = receive.recv_timeout(Duration::from_secs(15));
    let locked = Journal::open(&path, &spec()).is_err();
    let _ = child.kill();
    child.wait().unwrap();
    assert!(reached.unwrap());
    assert!(locked);
    let mut restored = Journal::open(&path, &spec()).unwrap();
    assert_eq!(restored.state().accounting().dispatched_attempts, 1);
    assert_eq!(restored.state().accounting().charged_work, 1);
    assert!(matches!(
        restored.state().attempts()[0].phase,
        Phase::Uncertain {
            cancellation_requested: false
        }
    ));
    let control = restored.state().attempts()[0].control.clone();
    assert!(
        restored
            .apply(Command::Dispatch {
                control: control.clone()
            })
            .is_err()
    );
    let output_path = path.with_extension("output.json");
    assert_eq!(output_path.exists(), after_output);
    if after_output {
        let response: Response = serde_json::from_slice(&fs::read(output_path).unwrap()).unwrap();
        let attempt = &restored.state().attempts()[0];
        worker::validate(
            &Dispatch {
                binding: attempt.dispatch.clone().unwrap(),
                input: attempt.request.input.clone(),
            },
            &response,
        )
        .unwrap();
        assert!(matches!(
            restored
                .apply(Command::Deliver {
                    response: response.clone()
                })
                .unwrap(),
            Receipt::Ignored { .. }
        ));
        let Receipt::Control { control } = restored
            .apply(Command::Reconcile { control, response })
            .unwrap()
        else {
            panic!()
        };
        assert!(matches!(
            restored.apply(Command::Collect { control }).unwrap(),
            Receipt::Collected {
                outcome: Outcome::Completed {
                    value: Value::Int(25)
                },
                ..
            }
        ));
        assert_eq!(restored.state().accounting().dispatched_attempts, 1);
        assert_eq!(restored.state().accounting().charged_work, 1);
    }
}
