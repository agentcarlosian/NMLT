import NMLT.Behavior.ResourceWorld

namespace NMLT.Artifact.FiniteAuthority

open NMLT.Behavior.ResourceBehavior NMLT.Behavior.ResourceWorld

variable {n f g : Nat}
variable (p q : ResourceProfile (Fin n) (Fin f) (Fin g))
variable [DecidablePred p.requires] [DecidablePred p.consumes]
  [DecidablePred p.transfers] [DecidablePred p.receives]
  [DecidablePred p.relies] [DecidablePred p.guarantees]
variable [DecidablePred q.requires] [DecidablePred q.consumes]
  [DecidablePred q.transfers] [DecidablePred q.receives]
  [DecidablePred q.relies] [DecidablePred q.guarantees]

instance enabledDecidable (actor : Bool) (world : AuthorityWorld (Fin n) Bool) :
    Decidable (Enabled actor p world) :=
  decidable_of_iff
    ((∀ c, p.requires c → world.owner c = some actor) ∧
     (∀ c, p.consumes c → world.owner c = some actor) ∧
     (∀ c, p.transfers c → world.owner c = some actor) ∧
     (∀ c, p.receives c → world.owner c ≠ some actor) ∧
     (∀ c, p.consumes c → ¬ p.transfers c) ∧
     (∀ c, p.consumes c → ¬ p.receives c) ∧
     (∀ c, p.transfers c → ¬ p.receives c))
    ⟨fun ⟨a,b,c,d,e,f,g⟩ => ⟨a,b,c,d,e,f,g⟩,
     fun h => ⟨h.requires,h.consumes,h.transfers,h.receivesFresh,
       h.consumeTransfer,h.consumeReceive,h.transferReceive⟩⟩

instance compatibleDecidable : Decidable (SynchronizationCompatible p q) :=
  decidable_of_iff
    ((∀ c, p.transfers c ↔ q.receives c) ∧
     (∀ c, q.transfers c ↔ p.receives c) ∧
     (∀ f, p.relies f → q.guarantees f) ∧
     (∀ f, q.relies f → p.guarantees f))
    ⟨fun ⟨a,b,c,d⟩ => ⟨a,b,c,d⟩,
     fun h => ⟨h.transfer,h.noReverseTransfer,h.senderRely,h.receiverRely⟩⟩

instance localStepDecidable (actor : Bool) (before after : AuthorityWorld (Fin n) Bool) :
    Decidable (LocalStep actor p before after) :=
  decidable_of_iff
    (Enabled actor p before ∧ (∀ c, ¬ p.transfers c) ∧ (∀ c, ¬ p.receives c) ∧
     (∀ c, p.consumes c → after.owner c = none) ∧
     (∀ c, ¬ p.consumes c → after.owner c = before.owner c))
    ⟨fun ⟨a,b,c,d,e⟩ => ⟨a,b,c,d,e⟩,
     fun h => ⟨h.enabled,h.noTransfer,h.noReceive,h.consumed,h.preserved⟩⟩

instance syncEnabledDecidable (left right : Bool) (world : AuthorityWorld (Fin n) Bool) :
    Decidable (SyncEnabled left right p q world) :=
  decidable_of_iff
    (left ≠ right ∧ Enabled left p world ∧ Enabled right q world ∧ SynchronizationCompatible p q)
    ⟨fun ⟨a,b,c,d⟩ => ⟨a,b,c,d⟩,
     fun h => ⟨h.ownersDistinct,h.leftEnabled,h.rightEnabled,h.compatible⟩⟩

instance syncStepDecidable (left right : Bool) (before after : AuthorityWorld (Fin n) Bool) :
    Decidable (SyncStep left right p q before after) :=
  decidable_of_iff
    (SyncEnabled left right p q before ∧
     (∀ c, p.consumes c → after.owner c = none) ∧
     (∀ c, q.consumes c → after.owner c = none) ∧
     (∀ c, p.transfers c → after.owner c = some right) ∧
     (∀ c, q.transfers c → after.owner c = some left) ∧
     (∀ c, ¬ p.consumes c → ¬ q.consumes c → ¬ p.transfers c → ¬ q.transfers c →
       after.owner c = before.owner c))
    ⟨fun ⟨a,b,c,d,e,f⟩ => ⟨a,b,c,d,e,f⟩,
     fun h => ⟨h.enabled,h.consumedLeft,h.consumedRight,
       h.transferredLeft,h.transferredRight,h.preserved⟩⟩

end NMLT.Artifact.FiniteAuthority
