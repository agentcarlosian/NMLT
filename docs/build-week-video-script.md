# NMLT Build Week video: recording and voice-over script

Target duration: 2 minutes 50 seconds.
Category: Developer Tools.
Primary command: ./judge-demo.sh --paced

## Production direction

Use one continuous real demo run as the visual spine. Cut to large source
close-ups and terminal callouts; do not leave judges reading a tiny split
screen. Keep the manual-abstraction boundary visible during every C/Rust shot.
Show Codex for at least ten seconds. End on the developer outcome, not Lean or
the multi-engine architecture.

## Timed script

### 0:00-0:15

Visual: Large C and Rust excerpts. Highlight the armed guard in C and the same
line missing from the Rust candidate. On-screen label: MANUALLY ABSTRACTED
SCENARIO.

Voice-over:

An AI-assisted port can compile, pass ordinary tests, and still lose one
workflow guard. Here the C implementation refuses dispatch until an attempt is
armed. The proposed Rust port drops that check. NMLT asks a narrow, reviewable
question: can the modeled workflow now dispatch while armed is false?

### 0:15-0:29

Visual: Extract the release bundle and run ./judge-demo.sh. On-screen label:
PREBUILT UBUNTU X86-64. NO BUILD. NO NETWORK.

Voice-over:

This is the prebuilt judge bundle. One command runs locally with no build and
no network. It checks the preserved workflow, the dropped-guard model, and a
stale-evidence control.

### 0:29-0:58

Visual: Let section one establish the manual boundary. Zoom to require armed in
guard-preserved.nmlt, then the MODEL CHECKED output, state count, transition
count, bounds, and four property names.

Voice-over:

The boundary is explicit. NMLT does not parse C or Rust and does not prove the
programs equivalent. I manually abstracted the relevant behavior into this
finite NMLT model. The preserved version requires the authorized phase and the
armed state before dispatch. The checker exhausts the reachable state space
within the displayed limits, and every listed property holds in that model.

### 0:58-1:30

Visual: Show the one-line model diff: remove require armed. Run the mutant.
Enlarge COUNTEREXAMPLE and animate the three witness states through authorize
and dispatch.

Voice-over:

Now I remove exactly the modeled armed guard to represent the faulty port. The
checker reports a counterexample, not a generic red test. Starting unarmed, the
workflow authorizes and then dispatches while armed remains false. The
state-by-state witness shows the exact transition a reviewer must fix. A found
trace is decisive for this authored model.

### 1:30-1:57

Visual: Show READBACK PASS, the saved binding, the one-byte-source-change
message, and STALE EVIDENCE REJECTED with old and current digests.

Voice-over:

A green result must not survive a source change. The demo saves a deterministic
report, replays it successfully against the exact model, then changes those
source bytes. The binding changes, so the readback gate rejects the earlier
result and applies no stale model-checked claim. This proves freshness for the
model bytes, not fidelity between the manual model and native source.

### 1:57-2:18

Visual: Full-screen boundary card: MANUAL MODEL, FINITE STATE SPACE, EXPLICIT
BOUNDS, NOT C-TO-RUST EQUIVALENCE.

Voice-over:

That assurance ceiling is intentional. NMLT currently verifies manually
authored finite behavior models. It does not automatically translate C to
Rust, prove memory safety, or establish general source equivalence. The result
is useful because its scope, limits, and counterexample are inspectable.

### 2:18-2:43

Visual: Show the actual Codex CLI thread, then a simple flow graphic: Ian
defines behavior and trust boundaries; Sol proposes implementation; Rust,
Python, Lean, and negative controls check the work.

Voice-over:

I built NMLT with Sol, GPT-5.6 running inside the OpenAI Codex CLI. Codex
accelerated the Rust frontend and checker, evidence validators, seeded defects,
Lean experiments, and recovery across languages. I directed the semantics,
workflow abstraction, architecture, and trust boundaries. Generated work was
treated as a proposal; deterministic checkers and adversarial controls decided
what was accepted. NMLT has no runtime LLM dependency.

### 2:43-2:55

Visual: Final JUDGE RESULT card, repository, release tag, and headline.

Voice-over:

For developers reviewing generated workflows and safety-sensitive changes,
NMLT turns this looks safe into either bounded, source-bound evidence or the
exact trace that breaks it. Run the complete judge path from the release below.

## Recording checklist

- Record at 1920 by 1080.
- Use a terminal font large enough to read on a laptop.
- Capture one uninterrupted ./judge-demo.sh --paced run.
- Add close-up cuts for the guard, counterexample, and stale-binding mismatch.
- Do not describe model_checked as proved safe.
- Do not claim native C or Rust parsing, translation, or equivalence.
- Replace the current Devpost video only after checking the final audio mix.
