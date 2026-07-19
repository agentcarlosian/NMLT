enum tPhase {
  PROPOSED,
  AUTHORIZED,
  DISPATCHED,
  SELECTED,
  REJECTED,
  INDETERMINATE
}

type tSnapshot = (
  phase: tPhase,
  bound: bool,
  armed: bool,
  dispatchCount: int,
  responseIntact: bool,
  evaluationPassed: bool,
  dispatchEnabled: bool
);

event eStep;
event eSnapshot: tSnapshot;

machine ProviderAttempt {
  var phase: tPhase;
  var bound: bool;
  var armed: bool;
  var dispatchCount: int;
  var responseIntact: bool;
  var evaluationPassed: bool;

  fun Snapshot(): tSnapshot {
    return (
      phase = phase,
      bound = bound,
      armed = armed,
      dispatchCount = dispatchCount,
      responseIntact = responseIntact,
      evaluationPassed = evaluationPassed,
      dispatchEnabled = phase == AUTHORIZED && bound && armed && dispatchCount == 0
    );
  }

  fun ObserveAndContinue() {
    announce eSnapshot, Snapshot();
    if (phase == SELECTED || phase == REJECTED || phase == INDETERMINATE) {
      raise halt;
    } else {
      send this, eStep;
    }
  }

  start state Running {
    entry {
      ObserveAndContinue();
    }

    on eStep do {
      if (phase == PROPOSED) {
        bound = true;
        phase = AUTHORIZED;
      } else if (phase == AUTHORIZED && !armed) {
        armed = true;
      } else if (phase == AUTHORIZED && bound && armed && dispatchCount == 0) {
        dispatchCount = 1;
        phase = DISPATCHED;
      } else if (phase == DISPATCHED) {
        if ($) {
          phase = INDETERMINATE;
        } else {
          responseIntact = true;
          if ($) {
            evaluationPassed = true;
            phase = SELECTED;
          } else {
            evaluationPassed = false;
            phase = REJECTED;
          }
        }
      }
      ObserveAndContinue();
    }
  }
}
