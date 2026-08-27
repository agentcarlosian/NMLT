/-
  Weak conditional congruence sketch for Paper 1 / Orbit A.

  Depends only on NMLT.Core.Transition (+ the ping/receive counterexample module).
  Status:
  * interface premises + necessity lemma for I-NO-HIDDEN-BOUNDARY;
  * isolation-reflection helper (I-CONNECT domain fragment);
  * two positive toys where product weak refinement IS checked;
  * packaged left-lift under InterfaceCompatible for the small LTS model
    (safety fragment: no grades / fairness / capabilities);
  * full RFC resource/rely extensions remain out of scope.
-/
import NMLT.Core.Transition
import NMLT.Counterexamples.CompositionCongruence

namespace NMLT.Behavior.WeakConditionalCongruence

open NMLT
open NMLT.Counterexamples.CompositionCongruence

/-- A concrete left label is connected in `K` if some peer label is linked. -/
def Connected {LeftLabel RightLabel : Type}
    (K : Connection LeftLabel RightLabel) (label : LeftLabel) : Prop :=
  ∃ right, K.linked label right

/-- I-NO-HIDDEN-BOUNDARY: a hidden concrete label must not be connected. -/
def NoHiddenBoundary {ConcreteLabel AbstractLabel Obs RightLabel : Type}
    {concrete : LTS ConcreteLabel Obs}
    {abstract : LTS AbstractLabel Obs}
    (hidden : ConcreteLabel → Bool)
    (mapLabel : ConcreteLabel → AbstractLabel)
    (_R : WeakRefines concrete abstract hidden mapLabel)
    (K : Connection ConcreteLabel RightLabel) : Prop :=
  ∀ label, hidden label = true → ¬ Connected K label

/-- Whole-wiring coverage (finite Connection form of RFC 0008 I-CONNECT). -/
def WiringCovered {ConcreteLeft AbstractLeft ConcreteRight AbstractRight : Type}
    (concrete : Connection ConcreteLeft ConcreteRight)
    (abstract : Connection AbstractLeft AbstractRight)
    (mapLeft : ConcreteLeft → AbstractLeft)
    (mapRight : ConcreteRight → AbstractRight) : Prop :=
  (∀ l r, concrete.linked l r → abstract.linked (mapLeft l) (mapRight r)) ∧
  (∀ l' r', abstract.linked l' r' →
    ∃ l r, concrete.linked l r ∧ mapLeft l = l' ∧ mapRight r = r')

/-- Domain fragment of I-CONNECT: independent concrete labels stay independent
    after the label map (needed for visible left-only product steps). -/
def IsolationReflects {ConcreteLeft AbstractLeft RightLabel : Type}
    (K_C : Connection ConcreteLeft RightLabel)
    (K_A : Connection AbstractLeft RightLabel)
    (mapLeft : ConcreteLeft → AbstractLeft) : Prop :=
  ∀ l, ¬ Connected K_C l → ¬ Connected K_A (mapLeft l)

/-- Lift a left-component state map across a fixed peer. -/
def liftMap {CState AState PState : Type}
    (μ : CState → AState) : (CState × PState) → (AState × PState) :=
  fun s => (μ s.1, s.2)

/-- Default composite hidden classifier: left/sync inherit the left hidden bit;
    peer-only steps are treated as visible in this safety fragment. -/
def compositeHiddenOf {ConcreteLabel RightLabel : Type}
    (hidden : ConcreteLabel → Bool) :
    ParallelLabel ConcreteLabel RightLabel → Bool
  | .left l => hidden l
  | .right _ => false
  | .sync l _ => hidden l

/-- Default composite label map. -/
def compositeMapOf {ConcreteLabel AbstractLabel RightLabel : Type}
    (mapLabel : ConcreteLabel → AbstractLabel) :
    ParallelLabel ConcreteLabel RightLabel → ParallelLabel AbstractLabel RightLabel
  | .left l => .left (mapLabel l)
  | .right r => .right r
  | .sync l r => .sync (mapLabel l) r

/-- Interface premises for weak left-lift (safety fragment; no grades/fairness). -/
structure InterfaceCompatible
    {ConcreteLabel AbstractLabel RightLabel ObsLeft ObsRight : Type}
    (concrete : LTS ConcreteLabel ObsLeft)
    (abstract : LTS AbstractLabel ObsLeft)
    (_peer : LTS RightLabel ObsRight)
    (hidden : ConcreteLabel → Bool)
    (mapLabel : ConcreteLabel → AbstractLabel)
    (R : WeakRefines concrete abstract hidden mapLabel)
    (K_C : Connection ConcreteLabel RightLabel)
    (K_A : Connection AbstractLabel RightLabel) where
  noHiddenBoundary : NoHiddenBoundary hidden mapLabel R K_C
  wiring : WiringCovered K_C K_A mapLabel id
  isolation : IsolationReflects K_C K_A mapLabel

/-- Injectivity of the left label map + wiring reflection ⇒ isolation. -/
theorem isolation_of_wiring_injective
    {ConcreteLeft AbstractLeft RightLabel : Type}
    {K_C : Connection ConcreteLeft RightLabel}
    {K_A : Connection AbstractLeft RightLabel}
    {mapLeft : ConcreteLeft → AbstractLeft}
    (wiring : WiringCovered K_C K_A mapLeft id)
    (inj : ∀ x y, mapLeft x = mapLeft y → x = y) :
    IsolationReflects K_C K_A mapLeft := by
  intro l notConn absConn
  rcases absConn with ⟨r', linkedA⟩
  rcases wiring.2 (mapLeft l) r' linkedA with ⟨l0, r0, linkedC, hl, hr⟩
  have : Connected K_C l0 := ⟨r0, linkedC⟩
  have eq : l0 = l := inj l0 l hl
  exact notConn (eq ▸ this)

/-- Open conjecture statement name (kept for Paper 1 / RFC cross-links).
    The safety-fragment package theorem below discharges this shape when
    `compositeHidden`/`compositeMap` are the defaults. -/
def WeakConditionalCongruenceStatement
    {ConcreteLabel AbstractLabel RightLabel ObsLeft ObsRight : Type}
    (concrete : LTS ConcreteLabel ObsLeft)
    (abstract : LTS AbstractLabel ObsLeft)
    (peer : LTS RightLabel ObsRight)
    (hidden : ConcreteLabel → Bool)
    (mapLabel : ConcreteLabel → AbstractLabel)
    (R : WeakRefines concrete abstract hidden mapLabel)
    (K_C : Connection ConcreteLabel RightLabel)
    (K_A : Connection AbstractLabel RightLabel)
    (_iface : InterfaceCompatible concrete abstract peer hidden mapLabel R K_C K_A)
    (compositeHidden : ParallelLabel ConcreteLabel RightLabel → Bool)
    (compositeMap :
      ParallelLabel ConcreteLabel RightLabel → ParallelLabel AbstractLabel RightLabel) :
    Prop :=
  Nonempty
    (WeakRefines
      (parallel concrete peer K_C)
      (parallel abstract peer K_A)
      compositeHidden
      compositeMap)

/-- Necessity: ping/receive violates I-NO-HIDDEN-BOUNDARY. -/
theorem pingReceive_violates_noHiddenBoundary :
    ¬ NoHiddenBoundary
        (concrete := concreteSender)
        (abstract := abstractSender)
        senderHidden id senderRefinement connection := by
  intro h
  have connected : Connected connection SenderLabel.ping :=
    ⟨ReceiverLabel.receive, ⟨rfl, rfl⟩⟩
  exact (h SenderLabel.ping rfl) connected

/-- Dual of `pingReceive_violates_noHiddenBoundary`: classifying the connected
    ping as visible against the original step-free abstract sender already
    breaks local `WeakRefines` (A has no ping step). `VisibleSync` repairs
    this only by changing A. -/
theorem visiblePing_breaks_senderRefinement :
    ¬ Nonempty
        (WeakRefines
          concreteSender abstractSender
          (fun _ => false)
          (id : SenderLabel → SenderLabel)) := by
  intro ⟨R⟩
  have hstep : concreteSender.step () .ping () := rfl
  exact R.visibleStep hstep rfl

/-- Same T3 systems under the default composite classifiers, with ping
    treated as visible. Canonical product statement is
    `visibleClassification_fails_on_hidden_ping_wire` in
    `Counterexamples/CompositionCongruence.lean`. -/
theorem visibleClassification_fails_default_lift :
    ¬ Nonempty
        (WeakRefines
          concreteComposite
          abstractComposite
          (compositeHiddenOf (fun _ => false))
          (compositeMapOf (id : SenderLabel → SenderLabel))) := by
  intro ⟨R⟩
  exact (R.visibleStep concreteSynchronization rfl).2.1

#print axioms visiblePing_breaks_senderRefinement
#print axioms visibleClassification_fails_default_lift

/-- Same connection is covered by the identity wiring map (sanity for I-CONNECT). -/
theorem pingReceive_wiring_id :
    WiringCovered connection connection (id : SenderLabel → SenderLabel) id := by
  refine ⟨?_, ?_⟩
  · intro l r hl
    exact hl
  · intro l' r' hl
    exact ⟨l', r', hl, rfl, rfl⟩

/-- Connected labels are never hidden under I-NO-HIDDEN-BOUNDARY. -/
theorem connected_not_hidden
    {ConcreteLabel AbstractLabel Obs RightLabel : Type}
    {concrete : LTS ConcreteLabel Obs}
    {abstract : LTS AbstractLabel Obs}
    {hidden : ConcreteLabel → Bool}
    {mapLabel : ConcreteLabel → AbstractLabel}
    {R : WeakRefines concrete abstract hidden mapLabel}
    {K : Connection ConcreteLabel RightLabel}
    (nhb : NoHiddenBoundary hidden mapLabel R K)
    {label : ConcreteLabel}
    (conn : Connected K label) :
    hidden label = false := by
  cases h : hidden label with
  | false => rfl
  | true => exact (nhb label h conn).elim

/-! ### Safety-fragment package: left-lift under InterfaceCompatible -/

def weakConditionalCongruence_safety
    {ConcreteLabel AbstractLabel RightLabel ObsLeft ObsRight : Type}
    {concrete : LTS ConcreteLabel ObsLeft}
    {abstract : LTS AbstractLabel ObsLeft}
    {peer : LTS RightLabel ObsRight}
    {hidden : ConcreteLabel → Bool}
    {mapLabel : ConcreteLabel → AbstractLabel}
    {R : WeakRefines concrete abstract hidden mapLabel}
    {K_C : Connection ConcreteLabel RightLabel}
    {K_A : Connection AbstractLabel RightLabel}
    (iface : InterfaceCompatible concrete abstract peer hidden mapLabel R K_C K_A) :
    WeakRefines
      (parallel concrete peer K_C)
      (parallel abstract peer K_A)
      (compositeHiddenOf hidden)
      (compositeMapOf mapLabel) where
  mapState := liftMap R.mapState
  init := by
    intro s hs
    exact ⟨R.init hs.1, hs.2⟩
  observe := by
    intro s
    cases s with
    | mk c d =>
      simp [parallel, liftMap, R.observe c]
  hiddenStep := by
    intro s label t hstep hhid
    match label with
    | .left l =>
        have hhidL : hidden l = true := hhid
        have notConn : ¬ Connected K_C l := iface.noHiddenBoundary l hhidL
        have ⟨hLeft, hPeer, _⟩ := hstep
        have mapEq := R.hiddenStep hLeft hhidL
        cases s; cases t
        simp [liftMap] at *
        exact Prod.ext mapEq hPeer
    | .right _ =>
        exact (Bool.noConfusion hhid)
    | .sync l r =>
        have hhidL : hidden l = true := hhid
        have ⟨linked, _, _⟩ := hstep
        have conn : Connected K_C l := ⟨r, linked⟩
        exact (iface.noHiddenBoundary l hhidL conn).elim
  visibleStep := by
    intro s label t hstep hvis
    match label with
    | .left l =>
        have hvisL : hidden l = false := hvis
        have ⟨hLeft, hPeer, notEx⟩ := hstep
        have notConn : ¬ Connected K_C l := by
          intro ⟨r, linked⟩
          exact notEx ⟨r, linked⟩
        have notConnA : ¬ Connected K_A (mapLabel l) :=
          iface.isolation l notConn
        have absStep := R.visibleStep hLeft hvisL
        refine ⟨absStep, hPeer, ?_⟩
        intro ⟨r, linkedA⟩
        exact notConnA ⟨r, linkedA⟩
    | .right r =>
        have ⟨hRight, hLeft, notEx⟩ := hstep
        refine ⟨hRight, ?_, ?_⟩
        · cases s; cases t
          simp [liftMap] at *
          exact congrArg R.mapState hLeft
        · intro ⟨l, linkedA⟩
          -- reflect abstract peer-link to a concrete link, then reuse notEx
          rcases iface.wiring.2 l r linkedA with ⟨l0, r0, linkedC, hl, hr⟩
          have : r0 = r := hr
          subst this
          exact notEx ⟨l0, linkedC⟩
    | .sync l r =>
        have ⟨linked, hLeft, hRight⟩ := hstep
        have conn : Connected K_C l := ⟨r, linked⟩
        have hvisL : hidden l = false := connected_not_hidden iface.noHiddenBoundary conn
        have absLeft := R.visibleStep hLeft hvisL
        have absLinked : K_A.linked (mapLabel l) r := by
          have := iface.wiring.1 l r linked
          simpa using this
        exact ⟨absLinked, absLeft, hRight⟩

theorem weakConditionalCongruence_safety_nonempty
    {ConcreteLabel AbstractLabel RightLabel ObsLeft ObsRight : Type}
    {concrete : LTS ConcreteLabel ObsLeft}
    {abstract : LTS AbstractLabel ObsLeft}
    {peer : LTS RightLabel ObsRight}
    {hidden : ConcreteLabel → Bool}
    {mapLabel : ConcreteLabel → AbstractLabel}
    {R : WeakRefines concrete abstract hidden mapLabel}
    {K_C : Connection ConcreteLabel RightLabel}
    {K_A : Connection AbstractLabel RightLabel}
    (iface : InterfaceCompatible concrete abstract peer hidden mapLabel R K_C K_A) :
    Nonempty
      (WeakRefines
        (parallel concrete peer K_C)
        (parallel abstract peer K_A)
        (compositeHiddenOf hidden)
        (compositeMapOf mapLabel)) :=
  ⟨weakConditionalCongruence_safety iface⟩

#print axioms weakConditionalCongruence_safety
#print axioms weakConditionalCongruence_safety_nonempty

/-! ### Positive toy A: visible ping/receive (same wiring as the counterexample) -/

namespace VisibleSync

/-- Abstract sender that *can* take ping (visible refinement target). -/
def abstractSenderVisible : LTS SenderLabel Bool where
  State := Unit
  init _ := True
  step _ label _ := label = .ping
  observe _ := false

def senderVisible (_ : SenderLabel) : Bool := false

def senderRefinementVisible :
    WeakRefines concreteSender abstractSenderVisible senderVisible id where
  mapState _ := ()
  init _ := trivial
  observe _ := rfl
  hiddenStep _ impossible := by nomatch impossible
  visibleStep hstep _ := hstep

def concreteCompositeVisible := parallel concreteSender receiver connection
def abstractCompositeVisible := parallel abstractSenderVisible receiver connection

theorem noHiddenBoundary_visible :
    NoHiddenBoundary senderVisible id senderRefinementVisible connection := by
  intro label hhid
  cases hhid

theorem isolation_visible :
    IsolationReflects connection connection (id : SenderLabel → SenderLabel) :=
  isolation_of_wiring_injective pingReceive_wiring_id (fun _ _ h => h)

def ifaceVisible :
    InterfaceCompatible
      concreteSender abstractSenderVisible receiver
      senderVisible id senderRefinementVisible connection connection where
  noHiddenBoundary := noHiddenBoundary_visible
  wiring := pingReceive_wiring_id
  isolation := isolation_visible

/-- Positive control: with visible ping, the same connection admits product
    weak refinement (contrast `noCompositeRefinement` for the hidden case). -/
theorem visibleSync_productRefinement :
    Nonempty
      (WeakRefines
        concreteCompositeVisible
        abstractCompositeVisible
        (compositeHiddenOf senderVisible)
        (compositeMapOf (id : SenderLabel → SenderLabel))) :=
  weakConditionalCongruence_safety_nonempty ifaceVisible

/-- The synchronized step is enabled and classified visible. -/
theorem visibleSync_step :
    concreteCompositeVisible.step ((), false) (.sync .ping .receive) ((), true) ∧
      compositeHiddenOf senderVisible (.sync SenderLabel.ping ReceiverLabel.receive) = false :=
  ⟨⟨⟨rfl, rfl⟩, rfl, ⟨rfl, rfl, rfl⟩⟩, rfl⟩

#print axioms visibleSync_productRefinement

end VisibleSync

/-! ### Positive toy B: empty wiring + hidden left tau -/

namespace EmptyWiringTau

inductive TauLabel
  | tau

def concreteTau : LTS TauLabel Unit where
  State := Nat
  init s := s = 0
  step before label after := label = .tau ∧ after = before + 1
  observe _ := ()

def abstractTau : LTS TauLabel Unit where
  State := Unit
  init _ := True
  step _ _ _ := False
  observe _ := ()

def peerIdle : LTS Unit Unit where
  State := Unit
  init _ := True
  step _ _ _ := False
  observe _ := ()

def emptyConn : Connection TauLabel Unit where
  linked _ _ := False

def tauHidden (_ : TauLabel) : Bool := true

def tauRefinement :
    WeakRefines concreteTau abstractTau tauHidden id where
  mapState _ := ()
  init _ := trivial
  observe _ := rfl
  hiddenStep _ _ := rfl
  visibleStep _ impossible := by nomatch impossible

theorem empty_wiring :
    WiringCovered emptyConn emptyConn (id : TauLabel → TauLabel) id := by
  refine ⟨?_, ?_⟩
  · intro _ _ hl
    exact hl
  · intro _ _ hl
    exact ⟨_, _, hl, rfl, rfl⟩

theorem empty_noHiddenBoundary :
    NoHiddenBoundary tauHidden id tauRefinement emptyConn := by
  intro label hhid ⟨r, linked⟩
  exact linked

theorem empty_isolation :
    IsolationReflects emptyConn emptyConn (id : TauLabel → TauLabel) :=
  isolation_of_wiring_injective empty_wiring (fun _ _ h => h)

def ifaceTau :
    InterfaceCompatible
      concreteTau abstractTau peerIdle tauHidden id tauRefinement emptyConn emptyConn where
  noHiddenBoundary := empty_noHiddenBoundary
  wiring := empty_wiring
  isolation := empty_isolation

theorem emptyWiring_productRefinement :
    Nonempty
      (WeakRefines
        (parallel concreteTau peerIdle emptyConn)
        (parallel abstractTau peerIdle emptyConn)
        (compositeHiddenOf tauHidden)
        (compositeMapOf (id : TauLabel → TauLabel))) :=
  weakConditionalCongruence_safety_nonempty ifaceTau

#print axioms emptyWiring_productRefinement

end EmptyWiringTau

/-! ### Negative control: WiringCovered + NHB, but IsolationReflects fails

  Non-injective `mapLabel` collapses an independent concrete `tau` onto the
  same abstract label as a connected `ping`. Whole-wiring coverage still
  holds (the abstract wire reflects to `ping`), and nothing is hidden, so
  NoHiddenBoundary is vacuous. IsolationReflects fails at `tau`, and the
  default product weak refinement is impossible: concrete can take an
  independent left `tau`, while the abstract product cannot take left-only
  `abs` because that label is connected.
-/

namespace CollidedAbstractWire

inductive ConcreteLabel
  | tau
  | ping

inductive AbstractLabel
  | abs

inductive PeerLabel
  | receive

def mapLabel : ConcreteLabel → AbstractLabel
  | .tau | .ping => .abs

def concreteLeft : LTS ConcreteLabel Bool where
  State := Unit
  init _ := True
  step _ _ _ := True
  observe _ := false

def abstractLeft : LTS AbstractLabel Bool where
  State := Unit
  init _ := True
  step _ _ _ := True
  observe _ := false

def peer : LTS PeerLabel Bool where
  State := Bool
  init state := state = false
  step before label after :=
    label = .receive ∧ before = false ∧ after = true
  observe state := state

def K_C : Connection ConcreteLabel PeerLabel where
  linked left right := left = .ping ∧ right = .receive

def K_A : Connection AbstractLabel PeerLabel where
  linked left right := left = .abs ∧ right = .receive

def hidden (_ : ConcreteLabel) : Bool := false

def refinement : WeakRefines concreteLeft abstractLeft hidden mapLabel where
  mapState _ := ()
  init _ := trivial
  observe _ := rfl
  hiddenStep _ impossible := by nomatch impossible
  visibleStep _ _ := trivial

theorem wiring_holds : WiringCovered K_C K_A mapLabel id := by
  refine ⟨?_, ?_⟩
  · intro l r hl
    match l, r, hl with
    | .ping, .receive, ⟨_, _⟩ =>
        exact ⟨rfl, rfl⟩
    | .tau, _, ⟨h, _⟩ =>
        nomatch h
  · intro l' r' hl
    match l', r', hl with
    | .abs, .receive, ⟨_, _⟩ =>
        exact ⟨.ping, .receive, ⟨rfl, rfl⟩, rfl, rfl⟩

theorem nhb_holds : NoHiddenBoundary hidden mapLabel refinement K_C := by
  intro _ hhid
  cases hhid

theorem isolation_fails :
    ¬ IsolationReflects K_C K_A mapLabel := by
  intro h
  have notConnTau : ¬ Connected K_C ConcreteLabel.tau := by
    intro ⟨_, linked⟩
    exact nomatch linked.1
  have connAbs : Connected K_A AbstractLabel.abs :=
    ⟨PeerLabel.receive, ⟨rfl, rfl⟩⟩
  have : mapLabel ConcreteLabel.tau = AbstractLabel.abs := rfl
  have notConn := h ConcreteLabel.tau notConnTau
  exact notConn (this ▸ connAbs)

theorem mapLabel_not_injective :
    ¬ (∀ x y : ConcreteLabel, mapLabel x = mapLabel y → x = y) := by
  intro inj
  exact nomatch (inj .tau .ping rfl)

def concreteComposite := parallel concreteLeft peer K_C
def abstractComposite := parallel abstractLeft peer K_A

theorem concrete_left_tau :
    concreteComposite.step ((), false) (.left .tau) ((), false) := by
  refine ⟨trivial, rfl, ?_⟩
  intro ⟨_, linked⟩
  exact nomatch linked.1

theorem abstract_left_abs_impossible
    {s t : Unit × Bool} :
    ¬ abstractComposite.step s (.left AbstractLabel.abs) t := by
  intro h
  exact h.2.2 ⟨PeerLabel.receive, ⟨rfl, rfl⟩⟩

theorem collided_noProductRefinement :
    ¬ Nonempty
        (WeakRefines
          concreteComposite
          abstractComposite
          (compositeHiddenOf (RightLabel := PeerLabel) hidden)
          (compositeMapOf (RightLabel := PeerLabel) mapLabel)) := by
  intro ⟨R⟩
  have hstep := concrete_left_tau
  have absStep :=
    R.visibleStep (label := .left .tau) hstep rfl
  exact abstract_left_abs_impossible absStep

theorem isolation_necessary_for_default_lift :
    NoHiddenBoundary hidden mapLabel refinement K_C ∧
      WiringCovered K_C K_A mapLabel id ∧
      ¬ IsolationReflects K_C K_A mapLabel ∧
      ¬ Nonempty
          (WeakRefines
            concreteComposite
            abstractComposite
            (compositeHiddenOf (RightLabel := PeerLabel) hidden)
            (compositeMapOf (RightLabel := PeerLabel) mapLabel)) :=
  ⟨nhb_holds, wiring_holds, isolation_fails, collided_noProductRefinement⟩

#print axioms isolation_fails
#print axioms collided_noProductRefinement
#print axioms isolation_necessary_for_default_lift

end CollidedAbstractWire

/-! ### Negative control: NHB + IsolationReflects, but WiringCovered fails

  An extra abstract wire sits on a label *outside* the image of `mapLabel`.
  Isolation still holds (the image name stays free). Coverage fails because
  that abstract wire has no concrete preimage. The concrete product can take
  an independent peer `receive`; the abstract product cannot, so the default
  lift is impossible.
-/

namespace ExtraAbstractWire

inductive ConcreteLabel
  | tau

inductive AbstractLabel
  | abs
  | extra

inductive PeerLabel
  | receive

def mapLabel : ConcreteLabel → AbstractLabel
  | .tau => .abs

def concreteLeft : LTS ConcreteLabel Bool where
  State := Unit
  init _ := True
  step _ _ _ := True
  observe _ := false

def abstractLeft : LTS AbstractLabel Bool where
  State := Unit
  init _ := True
  step _ _ _ := True
  observe _ := false

def peer : LTS PeerLabel Bool where
  State := Bool
  init state := state = false
  step before label after :=
    label = .receive ∧ before = false ∧ after = true
  observe state := state

def K_C : Connection ConcreteLabel PeerLabel where
  linked _ _ := False

def K_A : Connection AbstractLabel PeerLabel where
  linked left right := left = .extra ∧ right = .receive

def hidden (_ : ConcreteLabel) : Bool := false

def refinement : WeakRefines concreteLeft abstractLeft hidden mapLabel where
  mapState _ := ()
  init _ := trivial
  observe _ := rfl
  hiddenStep _ impossible := by nomatch impossible
  visibleStep _ _ := trivial

theorem nhb_holds : NoHiddenBoundary hidden mapLabel refinement K_C := by
  intro _ hhid
  cases hhid

theorem isolation_holds : IsolationReflects K_C K_A mapLabel := by
  intro l notConn
  match l with
  | .tau =>
      intro ⟨r, linked⟩
      -- mapLabel tau = abs, and K_A only links extra
      exact nomatch linked.1

theorem wiring_fails : ¬ WiringCovered K_C K_A mapLabel id := by
  intro h
  have absLink : K_A.linked AbstractLabel.extra PeerLabel.receive := ⟨rfl, rfl⟩
  rcases h.2 AbstractLabel.extra PeerLabel.receive absLink with ⟨l, r, linkedC, _, _⟩
  exact linkedC

theorem mapLabel_injective :
    ∀ x y : ConcreteLabel, mapLabel x = mapLabel y → x = y := by
  intro x y h
  match x, y with
  | .tau, .tau => rfl

def concreteComposite := parallel concreteLeft peer K_C
def abstractComposite := parallel abstractLeft peer K_A

theorem concrete_peer_receive :
    concreteComposite.step ((), false) (.right .receive) ((), true) := by
  refine ⟨⟨rfl, rfl, rfl⟩, rfl, ?_⟩
  intro ⟨_, linked⟩
  exact linked

theorem abstract_peer_receive_impossible
    {s t : Unit × Bool} :
    ¬ abstractComposite.step s (.right PeerLabel.receive) t := by
  intro h
  exact h.2.2 ⟨AbstractLabel.extra, ⟨rfl, rfl⟩⟩

theorem extraWire_noProductRefinement :
    ¬ Nonempty
        (WeakRefines
          concreteComposite
          abstractComposite
          (compositeHiddenOf (RightLabel := PeerLabel) hidden)
          (compositeMapOf (RightLabel := PeerLabel) mapLabel)) := by
  intro ⟨R⟩
  have hstep := concrete_peer_receive
  have absStep :=
    R.visibleStep (label := .right .receive) hstep rfl
  exact abstract_peer_receive_impossible absStep

theorem wiring_necessary_for_default_lift :
    NoHiddenBoundary hidden mapLabel refinement K_C ∧
      IsolationReflects K_C K_A mapLabel ∧
      ¬ WiringCovered K_C K_A mapLabel id ∧
      ¬ Nonempty
          (WeakRefines
            concreteComposite
            abstractComposite
            (compositeHiddenOf (RightLabel := PeerLabel) hidden)
            (compositeMapOf (RightLabel := PeerLabel) mapLabel)) :=
  ⟨nhb_holds, isolation_holds, wiring_fails, extraWire_noProductRefinement⟩

#print axioms isolation_holds
#print axioms extraWire_noProductRefinement
#print axioms wiring_necessary_for_default_lift

end ExtraAbstractWire

end NMLT.Behavior.WeakConditionalCongruence
