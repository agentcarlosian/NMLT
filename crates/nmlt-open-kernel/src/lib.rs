#![no_std]
#![forbid(unsafe_code)]

//! Bounded numeric decision kernel for M11 open-congruence certificates.
//!
//! The richer `nmlt-temporal` encoder owns names, diagnostics, and canonical
//! dictionaries. This crate receives only fixed-size numeric tables. It uses
//! no allocation, iterators, collection library, or unbounded loop so the
//! executed acceptance path can be translated to Lean by Aeneas.

pub const MAX_STATES: usize = 4;
pub const MAX_ACTIONS: usize = 4;
pub const MAX_PAYLOAD_VARIANTS: usize = 4;
pub const MAX_ATOMS: usize = 8;
pub const MAX_CONNECTIONS: usize = 4;
pub const NO_CHANNEL: u32 = u32::MAX;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct IndexTable {
    pub len: usize,
    pub values: [usize; MAX_ACTIONS],
}

impl IndexTable {
    pub const fn empty() -> Self {
        Self {
            len: 0,
            values: [0; MAX_ACTIONS],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StateMap {
    pub len: usize,
    pub values: [usize; MAX_STATES],
}

impl StateMap {
    pub const fn empty() -> Self {
        Self {
            len: 0,
            values: [0; MAX_STATES],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AtomTable {
    pub len: usize,
    pub values: [u32; MAX_ATOMS],
}

impl AtomTable {
    pub const fn empty() -> Self {
        Self {
            len: 0,
            values: [0; MAX_ATOMS],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PredicateTable {
    pub len: usize,
    pub values: [bool; MAX_PAYLOAD_VARIANTS],
}

impl PredicateTable {
    pub const fn empty() -> Self {
        Self {
            len: 0,
            values: [false; MAX_PAYLOAD_VARIANTS],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Grade {
    pub cost: u64,
    pub privacy: u64,
    pub energy: u64,
    pub uncertainty: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Resources {
    pub required: AtomTable,
    pub consumed: AtomTable,
    pub transferred: AtomTable,
    pub received: AtomTable,
    pub grade: Grade,
    pub rely: AtomTable,
    pub guarantees: AtomTable,
}

impl Resources {
    pub const fn empty() -> Self {
        Self {
            required: AtomTable::empty(),
            consumed: AtomTable::empty(),
            transferred: AtomTable::empty(),
            received: AtomTable::empty(),
            grade: Grade {
                cost: 0,
                privacy: 0,
                energy: 0,
                uncertainty: 0,
            },
            rely: AtomTable::empty(),
            guarantees: AtomTable::empty(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Action {
    /// `0 = internal`, `1 = input`, and `2 = output`.
    pub polarity: u8,
    pub channel: u32,
    pub assumption: PredicateTable,
    pub guarantee: PredicateTable,
    pub resources: Resources,
}

impl Action {
    pub const fn empty() -> Self {
        Self {
            polarity: 0,
            channel: NO_CHANNEL,
            assumption: PredicateTable::empty(),
            guarantee: PredicateTable::empty(),
            resources: Resources::empty(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct System {
    pub state_count: usize,
    pub action_count: usize,
    pub actions: [Action; MAX_ACTIONS],
    pub owned: AtomTable,
}

impl System {
    pub const fn empty() -> Self {
        Self {
            state_count: 0,
            action_count: 0,
            actions: [Action::empty(); MAX_ACTIONS],
            owned: AtomTable::empty(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Refinement {
    pub state_map: StateMap,
    pub action_map: IndexTable,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ConnectionTable {
    pub len: usize,
    pub left: [usize; MAX_CONNECTIONS],
    pub right: [usize; MAX_CONNECTIONS],
}

impl ConnectionTable {
    pub const fn empty() -> Self {
        Self {
            len: 0,
            left: [0; MAX_CONNECTIONS],
            right: [0; MAX_CONNECTIONS],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Congruence {
    pub payload_identity_present: bool,
    pub payload_variants_unique: bool,
    pub payload_cardinality: usize,
    pub concrete_left: System,
    pub abstract_left: System,
    pub concrete_right: System,
    pub abstract_right: System,
    pub left_refinement: Refinement,
    pub right_refinement: Refinement,
    pub concrete_wiring: ConnectionTable,
    pub abstract_wiring: ConnectionTable,
}

fn active8(len: usize, index: usize) -> bool {
    index < len
}

fn active16(len: usize, index: usize) -> bool {
    index < len
}

fn atom_contains(table: AtomTable, value: u32) -> bool {
    (active16(table.len, 0) && table.values[0] == value)
        || (active16(table.len, 1) && table.values[1] == value)
        || (active16(table.len, 2) && table.values[2] == value)
        || (active16(table.len, 3) && table.values[3] == value)
        || (active16(table.len, 4) && table.values[4] == value)
        || (active16(table.len, 5) && table.values[5] == value)
        || (active16(table.len, 6) && table.values[6] == value)
        || (active16(table.len, 7) && table.values[7] == value)
}

fn atom_slot_subset(left: AtomTable, right: AtomTable, index: usize) -> bool {
    !active16(left.len, index) || atom_contains(right, left.values[index])
}

fn atom_subset(left: AtomTable, right: AtomTable) -> bool {
    left.len <= MAX_ATOMS
        && right.len <= MAX_ATOMS
        && atom_slot_subset(left, right, 0)
        && atom_slot_subset(left, right, 1)
        && atom_slot_subset(left, right, 2)
        && atom_slot_subset(left, right, 3)
        && atom_slot_subset(left, right, 4)
        && atom_slot_subset(left, right, 5)
        && atom_slot_subset(left, right, 6)
        && atom_slot_subset(left, right, 7)
}

fn atom_slot_equal(left: AtomTable, right: AtomTable, index: usize) -> bool {
    !active16(left.len, index) || left.values[index] == right.values[index]
}

fn atom_equal(left: AtomTable, right: AtomTable) -> bool {
    left.len == right.len
        && left.len <= MAX_ATOMS
        && atom_slot_equal(left, right, 0)
        && atom_slot_equal(left, right, 1)
        && atom_slot_equal(left, right, 2)
        && atom_slot_equal(left, right, 3)
        && atom_slot_equal(left, right, 4)
        && atom_slot_equal(left, right, 5)
        && atom_slot_equal(left, right, 6)
        && atom_slot_equal(left, right, 7)
}

fn predicate_slot_subset(left: PredicateTable, right: PredicateTable, index: usize) -> bool {
    !left.values[index] || right.values[index]
}

fn predicate_subset(cardinality: usize, left: PredicateTable, right: PredicateTable) -> bool {
    left.len == cardinality
        && right.len == cardinality
        && cardinality <= MAX_PAYLOAD_VARIANTS
        && predicate_slot_subset(left, right, 0)
        && predicate_slot_subset(left, right, 1)
        && predicate_slot_subset(left, right, 2)
        && predicate_slot_subset(left, right, 3)
}

fn resources_compatible(concrete: Resources, abstract_resource: Resources) -> bool {
    atom_subset(concrete.required, abstract_resource.required)
        && atom_equal(concrete.consumed, abstract_resource.consumed)
        && atom_equal(concrete.transferred, abstract_resource.transferred)
        && atom_equal(concrete.received, abstract_resource.received)
        && concrete.grade.cost <= abstract_resource.grade.cost
        && concrete.grade.privacy <= abstract_resource.grade.privacy
        && concrete.grade.energy <= abstract_resource.grade.energy
        && concrete.grade.uncertainty <= abstract_resource.grade.uncertainty
        && atom_subset(concrete.rely, abstract_resource.rely)
        && atom_subset(abstract_resource.guarantees, concrete.guarantees)
}

fn action_compatible(cardinality: usize, concrete: Action, abstract_action: Action) -> bool {
    concrete.polarity == abstract_action.polarity
        && concrete.channel == abstract_action.channel
        && predicate_subset(cardinality, abstract_action.assumption, concrete.assumption)
        && predicate_subset(cardinality, concrete.guarantee, abstract_action.guarantee)
        && resources_compatible(concrete.resources, abstract_action.resources)
}

fn state_contains(map: StateMap, value: usize) -> bool {
    (active8(map.len, 0) && map.values[0] == value)
        || (active8(map.len, 1) && map.values[1] == value)
        || (active8(map.len, 2) && map.values[2] == value)
        || (active8(map.len, 3) && map.values[3] == value)
}

fn index_contains(map: IndexTable, value: usize) -> bool {
    (active8(map.len, 0) && map.values[0] == value)
        || (active8(map.len, 1) && map.values[1] == value)
        || (active8(map.len, 2) && map.values[2] == value)
        || (active8(map.len, 3) && map.values[3] == value)
}

fn state_target_valid(map: StateMap, target_count: usize, index: usize) -> bool {
    !active8(map.len, index) || map.values[index] < target_count
}

fn index_target_valid(map: IndexTable, target_count: usize, index: usize) -> bool {
    !active8(map.len, index) || map.values[index] < target_count
}

fn state_target_covered(map: StateMap, target_count: usize, target: usize) -> bool {
    target >= target_count || state_contains(map, target)
}

fn index_target_covered(map: IndexTable, target_count: usize, target: usize) -> bool {
    target >= target_count || index_contains(map, target)
}

fn index_unique_at(map: IndexTable, index: usize) -> bool {
    !active8(map.len, index)
        || (index == 0 || map.values[index] != map.values[0])
            && (index <= 1 || map.values[index] != map.values[1])
            && (index <= 2 || map.values[index] != map.values[2])
            && (index <= 3 || map.values[index] != map.values[3])
}

fn map_shapes_valid(concrete: System, abstract_system: System, refinement: Refinement) -> bool {
    concrete.state_count <= MAX_STATES
        && abstract_system.state_count <= MAX_STATES
        && concrete.action_count <= MAX_ACTIONS
        && abstract_system.action_count <= MAX_ACTIONS
        && refinement.state_map.len == concrete.state_count
        && refinement.action_map.len == concrete.action_count
        && state_target_valid(refinement.state_map, abstract_system.state_count, 0)
        && state_target_valid(refinement.state_map, abstract_system.state_count, 1)
        && state_target_valid(refinement.state_map, abstract_system.state_count, 2)
        && state_target_valid(refinement.state_map, abstract_system.state_count, 3)
        && index_target_valid(refinement.action_map, abstract_system.action_count, 0)
        && index_target_valid(refinement.action_map, abstract_system.action_count, 1)
        && index_target_valid(refinement.action_map, abstract_system.action_count, 2)
        && index_target_valid(refinement.action_map, abstract_system.action_count, 3)
        && state_target_covered(refinement.state_map, abstract_system.state_count, 0)
        && state_target_covered(refinement.state_map, abstract_system.state_count, 1)
        && state_target_covered(refinement.state_map, abstract_system.state_count, 2)
        && state_target_covered(refinement.state_map, abstract_system.state_count, 3)
        && index_target_covered(refinement.action_map, abstract_system.action_count, 0)
        && index_target_covered(refinement.action_map, abstract_system.action_count, 1)
        && index_target_covered(refinement.action_map, abstract_system.action_count, 2)
        && index_target_covered(refinement.action_map, abstract_system.action_count, 3)
        && index_unique_at(refinement.action_map, 0)
        && index_unique_at(refinement.action_map, 1)
        && index_unique_at(refinement.action_map, 2)
        && index_unique_at(refinement.action_map, 3)
}

fn action_slot_valid(
    cardinality: usize,
    concrete: System,
    abstract_system: System,
    map: IndexTable,
    index: usize,
) -> bool {
    !active8(concrete.action_count, index)
        || action_compatible(
            cardinality,
            concrete.actions[index],
            abstract_system.actions[map.values[index]],
        )
}

fn refinement_valid(
    cardinality: usize,
    concrete: System,
    abstract_system: System,
    refinement: Refinement,
) -> bool {
    map_shapes_valid(concrete, abstract_system, refinement)
        && atom_subset(concrete.owned, abstract_system.owned)
        && action_slot_valid(
            cardinality,
            concrete,
            abstract_system,
            refinement.action_map,
            0,
        )
        && action_slot_valid(
            cardinality,
            concrete,
            abstract_system,
            refinement.action_map,
            1,
        )
        && action_slot_valid(
            cardinality,
            concrete,
            abstract_system,
            refinement.action_map,
            2,
        )
        && action_slot_valid(
            cardinality,
            concrete,
            abstract_system,
            refinement.action_map,
            3,
        )
}

fn connection_contains(table: ConnectionTable, left: usize, right: usize) -> bool {
    (active8(table.len, 0) && table.left[0] == left && table.right[0] == right)
        || (active8(table.len, 1) && table.left[1] == left && table.right[1] == right)
        || (active8(table.len, 2) && table.left[2] == left && table.right[2] == right)
        || (active8(table.len, 3) && table.left[3] == left && table.right[3] == right)
}

fn concrete_edge_valid(
    concrete: ConnectionTable,
    abstract_wiring: ConnectionTable,
    left_map: IndexTable,
    right_map: IndexTable,
    index: usize,
) -> bool {
    !active8(concrete.len, index)
        || (concrete.left[index] < left_map.len
            && concrete.right[index] < right_map.len
            && connection_contains(
                abstract_wiring,
                left_map.values[concrete.left[index]],
                right_map.values[concrete.right[index]],
            ))
}

fn abstract_edge_covered(
    concrete: ConnectionTable,
    abstract_wiring: ConnectionTable,
    left_map: IndexTable,
    right_map: IndexTable,
    index: usize,
) -> bool {
    !active8(abstract_wiring.len, index)
        || (concrete_edge_maps_to(concrete, abstract_wiring, left_map, right_map, 0, index)
            || concrete_edge_maps_to(concrete, abstract_wiring, left_map, right_map, 1, index)
            || concrete_edge_maps_to(concrete, abstract_wiring, left_map, right_map, 2, index)
            || concrete_edge_maps_to(concrete, abstract_wiring, left_map, right_map, 3, index))
}

fn concrete_edge_maps_to(
    concrete: ConnectionTable,
    abstract_wiring: ConnectionTable,
    left_map: IndexTable,
    right_map: IndexTable,
    concrete_index: usize,
    abstract_index: usize,
) -> bool {
    active8(concrete.len, concrete_index)
        && concrete.left[concrete_index] < left_map.len
        && concrete.right[concrete_index] < right_map.len
        && left_map.values[concrete.left[concrete_index]] == abstract_wiring.left[abstract_index]
        && right_map.values[concrete.right[concrete_index]] == abstract_wiring.right[abstract_index]
}

fn wiring_valid(raw: Congruence) -> bool {
    raw.concrete_wiring.len <= MAX_CONNECTIONS
        && raw.abstract_wiring.len <= MAX_CONNECTIONS
        && raw.concrete_wiring.len == raw.abstract_wiring.len
        && concrete_edge_valid(
            raw.concrete_wiring,
            raw.abstract_wiring,
            raw.left_refinement.action_map,
            raw.right_refinement.action_map,
            0,
        )
        && concrete_edge_valid(
            raw.concrete_wiring,
            raw.abstract_wiring,
            raw.left_refinement.action_map,
            raw.right_refinement.action_map,
            1,
        )
        && concrete_edge_valid(
            raw.concrete_wiring,
            raw.abstract_wiring,
            raw.left_refinement.action_map,
            raw.right_refinement.action_map,
            2,
        )
        && concrete_edge_valid(
            raw.concrete_wiring,
            raw.abstract_wiring,
            raw.left_refinement.action_map,
            raw.right_refinement.action_map,
            3,
        )
        && abstract_edge_covered(
            raw.concrete_wiring,
            raw.abstract_wiring,
            raw.left_refinement.action_map,
            raw.right_refinement.action_map,
            0,
        )
        && abstract_edge_covered(
            raw.concrete_wiring,
            raw.abstract_wiring,
            raw.left_refinement.action_map,
            raw.right_refinement.action_map,
            1,
        )
        && abstract_edge_covered(
            raw.concrete_wiring,
            raw.abstract_wiring,
            raw.left_refinement.action_map,
            raw.right_refinement.action_map,
            2,
        )
        && abstract_edge_covered(
            raw.concrete_wiring,
            raw.abstract_wiring,
            raw.left_refinement.action_map,
            raw.right_refinement.action_map,
            3,
        )
}

/// Authoritative bounded acceptance decision. Every branch is finite and
/// explicit; oversized inputs must be rejected by construction or conversion.
#[must_use]
pub fn check(raw: Congruence) -> bool {
    raw.payload_identity_present
        && raw.payload_variants_unique
        && raw.payload_cardinality <= MAX_PAYLOAD_VARIANTS
        && refinement_valid(
            raw.payload_cardinality,
            raw.concrete_left,
            raw.abstract_left,
            raw.left_refinement,
        )
        && refinement_valid(
            raw.payload_cardinality,
            raw.concrete_right,
            raw.abstract_right,
            raw.right_refinement,
        )
        && wiring_valid(raw)
}

/// Accept only when the executed certificate is exactly the bounded source
/// snapshot supplied by the caller and that certificate satisfies the kernel.
/// This keeps the equality/readback decision inside the translated kernel.
#[must_use]
pub fn check_bound(expected: Congruence, raw: Congruence) -> bool {
    expected == raw && check(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action() -> Action {
        Action {
            polarity: 1,
            channel: 0,
            assumption: PredicateTable {
                len: 2,
                values: [true, false, false, false],
            },
            guarantee: PredicateTable {
                len: 2,
                values: [false; MAX_PAYLOAD_VARIANTS],
            },
            resources: Resources::empty(),
        }
    }

    fn system() -> System {
        let mut actions = [Action::empty(); MAX_ACTIONS];
        actions[0] = action();
        System {
            state_count: 1,
            action_count: 1,
            actions,
            owned: AtomTable::empty(),
        }
    }

    fn positive() -> Congruence {
        let refinement = Refinement {
            state_map: StateMap {
                len: 1,
                values: [0; MAX_STATES],
            },
            action_map: IndexTable {
                len: 1,
                values: [0; MAX_ACTIONS],
            },
        };
        let wiring = ConnectionTable {
            len: 1,
            left: [0; MAX_CONNECTIONS],
            right: [0; MAX_CONNECTIONS],
        };
        Congruence {
            payload_identity_present: true,
            payload_variants_unique: true,
            payload_cardinality: 2,
            concrete_left: system(),
            abstract_left: system(),
            concrete_right: system(),
            abstract_right: system(),
            left_refinement: refinement,
            right_refinement: refinement,
            concrete_wiring: wiring,
            abstract_wiring: wiring,
        }
    }

    #[test]
    fn accepts_positive_certificate() {
        assert!(check(positive()));
    }

    #[test]
    fn rejects_duplicate_action_target() {
        let mut raw = positive();
        raw.concrete_left.action_count = 2;
        raw.abstract_left.action_count = 2;
        raw.concrete_left.actions[1] = action();
        raw.abstract_left.actions[1] = action();
        raw.left_refinement.action_map.len = 2;
        raw.left_refinement.action_map.values[1] = 0;
        assert!(!check(raw));
    }

    #[test]
    fn rejects_nonmonotone_grade() {
        let mut raw = positive();
        raw.concrete_left.actions[0].resources.grade.cost = 2;
        raw.abstract_left.actions[0].resources.grade.cost = 1;
        assert!(!check(raw));
    }

    #[test]
    fn rejects_unreflected_wiring() {
        let mut raw = positive();
        raw.abstract_wiring.len = 2;
        raw.abstract_wiring.left[1] = 0;
        raw.abstract_wiring.right[1] = 0;
        assert!(!check(raw));
    }

    #[test]
    fn bound_check_rejects_structural_substitution() {
        let expected = positive();
        let mut substituted = expected;
        substituted.concrete_left.actions[0].channel = 1;
        assert!(!check_bound(expected, substituted));
    }

    #[test]
    fn bound_check_accepts_exact_valid_certificate() {
        let expected = positive();
        assert!(check_bound(expected, expected));
    }
}
