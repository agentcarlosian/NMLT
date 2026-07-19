use nmlt_certificate_tools::{measure, simplify};
use nmlt_elaborate::elaborate;
use nmlt_hir::{project_source_module, resolve_modules};
use nmlt_kernel::{RawCertificate, check};

const SOURCE: &str = concat!(
    "system Toggle {\n",
    " state ready: Bool = false\n",
    " action set(next_value: Bool) {\n",
    "  require next_value\n",
    "  set ready = next_value\n",
    " }\n",
    " safety Safe = always(ready or not enabled(set))\n",
    " observe ready\n",
    "}\n",
);

#[test]
fn pruning_is_measured_but_kernel_replay_remains_authoritative() {
    let projected = project_source_module("Toggle", "src/toggle.nmlt", SOURCE.as_bytes());
    let hir = resolve_modules(vec![projected]).unwrap();
    let artifact = elaborate(&hir).unwrap();
    let accepted = RawCertificate::from_artifact(&artifact);

    let mut overcomplete = accepted.clone();
    let mut extra = overcomplete.derivations[0].clone();
    extra.claimed_digest = [0xff; 32];
    overcomplete.derivations.push(extra);
    overcomplete.recompute_claimed_certificate_digest();

    let before = measure(&overcomplete).unwrap();
    assert_eq!(before.unreachable_nodes, 1);
    assert!(check(&hir, artifact.core_program(), &overcomplete).is_err());

    let (simplified, report) = simplify(overcomplete).unwrap();
    assert_eq!(report.removed_nodes, 1);
    assert_eq!(report.after.unreachable_nodes, 0);
    assert!(report.after.canonical_bytes < report.before.canonical_bytes);
    assert!(check(&hir, artifact.core_program(), &simplified).is_ok());
}
