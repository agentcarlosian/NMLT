import NMLT.Behavior.OpenKernelExecution
import Mathlib.Data.List.Nodup

namespace NMLT.Behavior.OpenKernelReadback

open Aeneas
open NMLT.OpenKernel
open NMLT.Behavior.OpenKernelExecution

/-- String dictionary carried beside the numeric kernel input. Duplicate atoms
would make numeric identifiers ambiguous, so canonical dictionaries are
required to be duplicate-free. -/
structure CanonicalDictionary where
  atoms : List String
  nodup : atoms.Nodup

def DecodesAtom (dictionary : CanonicalDictionary) (id : Nat) (atom : String) : Prop :=
  dictionary.atoms[id]? = some atom

theorem decode_functional (dictionary : CanonicalDictionary) {id : Nat} {left right : String}
    (leftDecodes : DecodesAtom dictionary id left)
    (rightDecodes : DecodesAtom dictionary id right) : left = right := by
  simp only [DecodesAtom] at leftDecodes rightDecodes
  rw [leftDecodes] at rightDecodes
  exact Option.some.inj rightDecodes

theorem decode_injective (dictionary : CanonicalDictionary) {leftId rightId : Nat} {atom : String}
    (leftDecodes : DecodesAtom dictionary leftId atom)
    (rightDecodes : DecodesAtom dictionary rightId atom) : leftId = rightId := by
  simp only [DecodesAtom] at leftDecodes rightDecodes
  obtain ⟨leftBound, leftValue⟩ := List.getElem?_eq_some_iff.mp leftDecodes
  obtain ⟨rightBound, rightValue⟩ := List.getElem?_eq_some_iff.mp rightDecodes
  exact dictionary.nodup.getElem_inj_iff.mp (leftValue.trans rightValue.symm)

def activeAtomIds (table : AtomTable) : List Nat :=
  (table.values.v.take table.len.val).map (fun id => id.val)

def resourceAtomIds (resources : Resources) : List Nat :=
  activeAtomIds resources.required ++
  activeAtomIds resources.consumed ++
  activeAtomIds resources.transferred ++
  activeAtomIds resources.received ++
  activeAtomIds resources.rely ++
  activeAtomIds resources.guarantees

def actionAtomIds (action : Action) : List Nat :=
  (if action.channel = NO_CHANNEL then [] else [action.channel.val]) ++
  resourceAtomIds action.resources

def systemAtomIds (system : System) : List Nat :=
  activeAtomIds system.owned ++
  (system.actions.v.take system.action_count.val).flatMap actionAtomIds

def referencedAtomIds (raw : Congruence) : List Nat :=
  systemAtomIds raw.concrete_left ++
  systemAtomIds raw.abstract_left ++
  systemAtomIds raw.concrete_right ++
  systemAtomIds raw.abstract_right

/-- Semantic readback condition checked after rich strings have been interned:
every active numeric identifier used by the kernel decodes through the unique
dictionary carried with that execution. -/
def ReadbackComplete (dictionary : CanonicalDictionary) (raw : Congruence) : Prop :=
  ∀ id ∈ referencedAtomIds raw, ∃ atom, DecodesAtom dictionary id atom

structure VerifiedExecution (dictionary : CanonicalDictionary) (raw : Congruence) : Prop where
  execution : ExecutionContract raw
  readback : ReadbackComplete dictionary raw

/-- Kernel acceptance and complete dictionary readback compose without adding
an assumption about the meaning of any numeric identifier. -/
theorem check_accepts_with_readback (dictionary : CanonicalDictionary) (raw : Congruence)
    (accepted : check raw = .ok true) (readback : ReadbackComplete dictionary raw) :
    VerifiedExecution dictionary raw :=
  ⟨check_accepts_implies_contract raw accepted, readback⟩

theorem referenced_id_in_dictionary (dictionary : CanonicalDictionary) (raw : Congruence)
    (verified : VerifiedExecution dictionary raw) {id : Nat}
    (referenced : id ∈ referencedAtomIds raw) : id < dictionary.atoms.length := by
  obtain ⟨atom, decodes⟩ := verified.readback id referenced
  exact List.getElem?_eq_some_iff.mp decodes |>.1

#print axioms decode_functional
#print axioms decode_injective
#print axioms check_accepts_with_readback
#print axioms referenced_id_in_dictionary

end NMLT.Behavior.OpenKernelReadback
