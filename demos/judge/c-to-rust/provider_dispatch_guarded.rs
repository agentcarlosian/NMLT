#![allow(dead_code)]

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Proposed,
    Authorized,
    Dispatched,
}

struct ProviderAttempt {
    phase: Phase,
    armed: bool,
    dispatched: bool,
}

impl ProviderAttempt {
    fn dispatch(&mut self) -> bool {
        if self.phase != Phase::Authorized {
            return false;
        }
        if !self.armed {
            return false;
        }

        self.dispatched = true;
        self.phase = Phase::Dispatched;
        true
    }
}
