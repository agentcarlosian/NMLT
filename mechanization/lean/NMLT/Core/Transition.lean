namespace NMLT

/-- A deliberately small labelled transition system used to falsify the
    candidate refinement rule before the full NMLT semantics is mechanized. -/
structure LTS (Label Obs : Type) where
  State : Type
  init : State -> Prop
  step : State -> Label -> State -> Prop
  observe : State -> Obs

/-- The RFC 0001 one-step weak forward simulation candidate. A hidden concrete
    step maps to equality; a visible step maps to exactly one abstract step. -/
structure WeakRefines {ConcreteLabel AbstractLabel Obs : Type}
    (concrete : LTS ConcreteLabel Obs) (abstract : LTS AbstractLabel Obs)
    (hidden : ConcreteLabel -> Bool) (mapLabel : ConcreteLabel -> AbstractLabel) where
  mapState : concrete.State -> abstract.State
  init : forall {s}, concrete.init s -> abstract.init (mapState s)
  observe : forall s, concrete.observe s = abstract.observe (mapState s)
  hiddenStep : forall {s label t}, concrete.step s label t -> hidden label = true ->
    mapState s = mapState t
  visibleStep : forall {s label t}, concrete.step s label t -> hidden label = false ->
    abstract.step (mapState s) (mapLabel label) (mapState t)

/-- Labels of an interleaving or synchronized product. -/
inductive ParallelLabel (LeftLabel RightLabel : Type)
  | left : LeftLabel -> ParallelLabel LeftLabel RightLabel
  | right : RightLabel -> ParallelLabel LeftLabel RightLabel
  | sync : LeftLabel -> RightLabel -> ParallelLabel LeftLabel RightLabel

/-- A connection says which left and right labels must synchronize. -/
structure Connection (LeftLabel RightLabel : Type) where
  linked : LeftLabel -> RightLabel -> Prop

/-- Product composition. A linked label cannot step independently; it must
    synchronize with its peer. This is the fragment needed by the Phase-1
    congruence counterexample. -/
def parallel {LeftLabel RightLabel LeftObs RightObs : Type}
    (left : LTS LeftLabel LeftObs) (right : LTS RightLabel RightObs)
    (connection : Connection LeftLabel RightLabel) :
    LTS (ParallelLabel LeftLabel RightLabel) (LeftObs × RightObs) where
  State := left.State × right.State
  init state := left.init state.1 ∧ right.init state.2
  observe state := (left.observe state.1, right.observe state.2)
  step before label after :=
    match label with
    | .left leftLabel =>
        left.step before.1 leftLabel after.1 ∧ before.2 = after.2 ∧
          ¬ Exists fun rightLabel => connection.linked leftLabel rightLabel
    | .right rightLabel =>
        right.step before.2 rightLabel after.2 ∧ before.1 = after.1 ∧
          ¬ Exists fun leftLabel => connection.linked leftLabel rightLabel
    | .sync leftLabel rightLabel =>
        connection.linked leftLabel rightLabel ∧
          left.step before.1 leftLabel after.1 ∧
          right.step before.2 rightLabel after.2

end NMLT
