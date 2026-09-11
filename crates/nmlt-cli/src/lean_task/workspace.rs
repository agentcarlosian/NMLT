//! Ordinary Lake workspaces for Lean LSP/REPL editing of a bound proof.
use super::{Result, Run, Task, err, retain_cli, source, write_json, write_new};
use nmlt_runtime::lean;
use serde_json::json;
use std::path::Path;
use std::process::Command;

pub(super) fn run(task: Task, executable: &Path, output: &Path, origin: &Path) -> Result<()> {
    let digest = task.digest()?;
    let toolchain = lean::Toolchain::open(executable).map_err(err)?;
    if *toolchain.identity() != task.lean {
        return Err("Lean installation differs from the bound task".into());
    }
    let mut run = Run::new(output, executable)?;
    task.prepare(origin, &mut run)?;
    if run.target(&task.manifest)? != task.target {
        return Err("editor target differs from the bound task".into());
    }
    // The build directory becomes a standalone editor package. Imported native
    // packages keep their original configuration in the adjacent source copies.
    let manifest = if let Some(lake) = &task.lake {
        lake.verify_run(&run)?;
        lake.editor_manifest()?
    } else {
        json!({"version":"1.2.0", "packagesDir":".lake/packages", "packages":[], "name":"nmlt_editor", "lakeDir":".lake"})
    };
    let mut configuration =
        String::from("name = \"nmlt_editor\"\ndefaultTargets = [\"NMLTEditor\"]\n\n");
    if let Some(root) = manifest["packages"].as_array().and_then(|a| a.first()) {
        configuration.push_str(&format!(
            "[[require]]\nname = {}\npath = \"../lake-project/root\"\n\n",
            root["name"]
        ));
    }
    let mut roots = vec!["NMLTTask", "NMLTProof"];
    if task.lake.is_none() {
        roots.extend(task.sources.iter().map(|s| s.module.as_str()));
    }
    configuration.push_str(&format!(
        "[[lean_lib]]\nname = \"NMLTEditor\"\nroots = {}\n",
        serde_json::to_string(&roots).map_err(err)?
    ));
    write_new(&run.build.join("lakefile.toml"), configuration.as_bytes())?;
    write_json(&run.build.join("lake-manifest.json"), &manifest)?;
    write_new(
        &run.build.join("lean-toolchain"),
        format!("leanprover/lean4:v{}\n", lean::version()).as_bytes(),
    )?;
    write_new(
        &run.build.join("NMLTProof.lean"),
        source::proof(&task.target, "by\n  sorry")?.as_bytes(),
    )?;
    let bin = executable
        .parent()
        .ok_or("missing Lean executable directory")?;
    let mut command = Command::new(bin.join(if cfg!(windows) { "lake.exe" } else { "lake" }));
    command
        .env_clear()
        .current_dir(&run.build)
        .env("PATH", bin)
        .env("LEAN_NUM_THREADS", "1")
        .env("LEAN_STACK_SIZE_KB", lean::STACK_KIB)
        .env("MIMALLOC_ARENA_RESERVE", lean::ARENA_KIB);
    for key in ["SystemRoot", "WINDIR", "TEMP", "TMP"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command.args([
        "--no-cache",
        "--rehash",
        "--quiet",
        "build",
        "+NMLTProof:olean",
    ]);
    // This is a native Lake build even when the task originally used explicit
    // local modules. Lake and its Lean child share the OS job's memory bound.
    run.project_processes = true;
    run.execute("editor-lake-build", command)?;
    toolchain.verify_unchanged().map_err(err)?;
    if let Some(lake) = &task.lake {
        lake.verify_run(&run)?;
    }
    retain_cli(&run.directory, &task.implementation_sha256)?;
    write_json(&run.directory.join("task.json"), &task)?;
    write_new(
        &run.directory.join("task.sha256"),
        format!("{digest}\n").as_bytes(),
    )?;
    write_json(
        &run.directory.join("workspace.json"),
        &json!({"schema":"nmlt-lean-workspace-v1", "status":"context_only", "assurance":"none", "task_sha256":digest, "editor_root":"build", "proof_file":"build/NMLTProof.lean", "process_policy":run.policy, "stages":run.stages}),
    )?;
    write_new(&run.directory.join("README.md"), format!("# Lean proof workspace\n\nTask: `{digest}`. Status: **unchecked draft**.\n\nOpen `build` as a normal Lean project in VS Code or another Lean LSP client. The pinned `lake serve` supplies diagnostics, goals and hover. Replace `sorry` in `build/NMLTProof.lean` with a supported proof body. `lake build` is editing feedback; acceptance requires the NMLT proof pipeline.\n\nUse the retained CLI's `lean-task candidate --task task.json --task-sha256 {digest} --proof build/NMLTProof.lean --output candidate.json`, then `lean-task prove` with that candidate and your pinned tools. Candidate creation checks that the generated target wrapper is unchanged. Sources or target statements can be changed only by binding an explicit task revision; this draft does not update earlier acceptance.\n").as_bytes())?;
    println!(
        "workspace: {}\nstatus: context_only\ntask: {digest}",
        run.build.display()
    );
    Ok(())
}
