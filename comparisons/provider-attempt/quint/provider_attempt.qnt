module provider_attempt {
  type Phase =
    | Proposed
    | Authorized
    | Dispatched
    | Selected
    | Rejected
    | Indeterminate

  var phase: Phase
  var bound: bool
  var armed: bool
  var dispatchCount: int
  var responseIntact: bool
  var evaluationPassed: bool

  action init = all {
    phase' = Proposed,
    bound' = false,
    armed' = false,
    dispatchCount' = 0,
    responseIntact' = false,
    evaluationPassed' = false,
  }

  action bind = all {
    phase == Proposed,
    phase' = Authorized,
    bound' = true,
    armed' = armed,
    dispatchCount' = dispatchCount,
    responseIntact' = responseIntact,
    evaluationPassed' = evaluationPassed,
  }

  action arm = all {
    phase == Authorized,
    bound,
    phase' = phase,
    bound' = bound,
    armed' = true,
    dispatchCount' = dispatchCount,
    responseIntact' = responseIntact,
    evaluationPassed' = evaluationPassed,
  }

  action dispatch = all {
    phase == Authorized,
    bound,
    armed,
    dispatchCount == 0,
    phase' = Dispatched,
    bound' = bound,
    armed' = armed,
    dispatchCount' = 1,
    responseIntact' = responseIntact,
    evaluationPassed' = evaluationPassed,
  }

  action confirmPass = all {
    phase == Dispatched,
    phase' = Selected,
    bound' = bound,
    armed' = armed,
    dispatchCount' = dispatchCount,
    responseIntact' = true,
    evaluationPassed' = true,
  }

  action confirmFail = all {
    phase == Dispatched,
    phase' = Rejected,
    bound' = bound,
    armed' = armed,
    dispatchCount' = dispatchCount,
    responseIntact' = true,
    evaluationPassed' = false,
  }

  action loseResponse = all {
    phase == Dispatched,
    phase' = Indeterminate,
    bound' = bound,
    armed' = armed,
    dispatchCount' = dispatchCount,
    responseIntact' = responseIntact,
    evaluationPassed' = evaluationPassed,
  }

  action step = any {
    bind,
    arm,
    dispatch,
    confirmPass,
    confirmFail,
    loseResponse,
  }

  val typeOk = dispatchCount >= 0 and dispatchCount <= 1
  val dispatchRequiresArm = dispatchCount > 0 implies (bound and armed)
  val selectionRequiresPassingEvidence =
    phase == Selected implies (responseIntact and evaluationPassed)
  val atMostOneDispatch = dispatchCount <= 1
  val noBlindReplay = phase == Indeterminate implies dispatchCount == 1
}
