import NMLT.Behavior.OpenEncodingCorrespondence

namespace NMLT.Behavior.OpenSourceReadback

open NMLT.Behavior.OpenEncodingCorrespondence

/-- The Rust boundary checks every source field independently. Equality of the
canonical representations is the Lean meaning of a successful complete
readback. -/
def ReadbackExact (source encoded : RawCongruence) : Prop :=
  encoded = source

/-- An accepted canonical certificate whose complete readback equals the source
snapshot transports the full implementation contract back to that source. -/
theorem accepted_exact_readback_contract
    (source encoded : RawCongruence)
    (maps : TypedMaps encoded)
    (accepted : check encoded maps = true)
    (readback : ReadbackExact source encoded) :
    ∃ sourceMaps : TypedMaps source,
      ImplementationContract source sourceMaps := by
  unfold ReadbackExact at readback
  subst source
  exact ⟨maps, accepted_implementation_contract encoded maps accepted⟩

/-- Exact readback cannot identify two different source snapshots. -/
theorem exact_readback_functional (source left right : RawCongruence)
    (leftExact : ReadbackExact source left)
    (rightExact : ReadbackExact source right) : left = right := by
  exact leftExact.trans rightExact.symm

#print axioms accepted_exact_readback_contract
#print axioms exact_readback_functional

end NMLT.Behavior.OpenSourceReadback
