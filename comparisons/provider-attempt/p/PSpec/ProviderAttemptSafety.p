spec ProviderAttemptSafety observes eSnapshot {
  start state Checking {
    on eSnapshot do (snapshot: tSnapshot) {
      assert snapshot.dispatchCount >= 0 && snapshot.dispatchCount <= 1,
        "dispatch count must remain in 0..1";
      assert snapshot.dispatchCount == 0 || (snapshot.bound && snapshot.armed),
        "dispatch requires a bound and armed attempt";
      assert snapshot.phase != SELECTED ||
        (snapshot.responseIntact && snapshot.evaluationPassed),
        "selection requires intact passing evidence";
      assert snapshot.phase != INDETERMINATE || !snapshot.dispatchEnabled,
        "indeterminate must not enable another dispatch";
    }
  }
}
