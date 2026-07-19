use nmlt_agent::evaluate_held_out_suite;

#[test]
fn assisted_completion_improves_without_authority_loss() {
    let report = evaluate_held_out_suite().unwrap();
    assert_eq!(report.metrics.task_count, 3);
    assert_eq!(report.metrics.baseline_completed, 0);
    assert_eq!(report.metrics.assisted_completed, 3);
    assert_eq!(report.metrics.semantic.assisted_completed, 1);
    assert_eq!(report.metrics.trusted_modification_attempts, 21);
    assert_eq!(report.metrics.trusted_modification_rejections, 21);
    assert_eq!(report.metrics.negative_controls_retained, 3);
    assert_eq!(report.metrics.negative_controls_killed, 3);
    assert_eq!(report.metrics.unknown_results_promoted, 0);
    assert_eq!(report.metrics.conflict_results_promoted, 0);
    assert!(!report.expected_patches_exposed);
    assert!(report.artifact_graph_readback_verified);
    assert!(report.tasks.iter().all(|task| {
        task.completed && task.trusted_identities_unchanged && task.localized_edit_count == 1
    }));
}

#[test]
fn report_is_valid_json_shape_without_expected_patch_material() {
    let report = evaluate_held_out_suite().unwrap();
    let json = report.to_json();
    assert!(json.starts_with('{') && json.ends_with('}'));
    assert!(json.contains("\"expected_patches_exposed\":false"));
    assert!(!json.contains("\"expected_patch\":"));
    assert!(!json.contains("\"gold_patch\":"));
}
