#include <stdbool.h>

enum Phase {
    PHASE_PROPOSED,
    PHASE_AUTHORIZED,
    PHASE_DISPATCHED
};

struct ProviderAttempt {
    enum Phase phase;
    bool armed;
    bool dispatched;
};

bool provider_dispatch(struct ProviderAttempt *attempt) {
    if (attempt->phase != PHASE_AUTHORIZED) {
        return false;
    }
    if (!attempt->armed) {
        return false;
    }

    attempt->dispatched = true;
    attempt->phase = PHASE_DISPATCHED;
    return true;
}
