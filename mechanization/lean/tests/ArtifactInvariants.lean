import NMLT.Artifact.SemanticClosure

open Lean
open NMLT.Artifact.BehaviorCore
open NMLT.Artifact.SemanticClosure

private def check (condition : Bool) (message : String) : IO Unit := do
  unless condition do throw (IO.userError message)

private structure EncodedTerm where
  ast : Json
  text : String

private def boolTerm (value : Bool) : EncodedTerm :=
  ⟨Json.mkObj [("kind", toJson "bool"), ("type", toJson "Bool"),
    ("value", toJson value)], if value then "true" else "false"⟩

private def unitTerm : EncodedTerm :=
  ⟨Json.mkObj [("kind", toJson "unit"), ("type", toJson "Unit")], "unit"⟩

private def enumTerm (typeName constructor : String) : EncodedTerm :=
  ⟨Json.mkObj [("kind", toJson "enum"), ("type", toJson typeName),
    ("constructor", toJson constructor)], constructor⟩

private def readTerm (typeName : String) : EncodedTerm :=
  ⟨Json.mkObj [("kind", toJson "read"), ("type", toJson typeName),
    ("field", toJson "current")], "current"⟩

private def notTerm (value : EncodedTerm) : EncodedTerm :=
  ⟨Json.mkObj [("kind", toJson "not"), ("type", toJson "Bool"),
    ("value", value.ast)], s!"!{value.text}"⟩

private def equalTerm (left right : EncodedTerm) : EncodedTerm :=
  ⟨Json.mkObj [("kind", toJson "equal"), ("type", toJson "Bool"),
    ("left", left.ast), ("right", right.ast)], s!"{left.text} == {right.text}"⟩

private def emptyObject : Json := Json.mkObj []

private def emptyResources : Json := Json.mkObj [
  ("requires", toJson ([] : List String)), ("consumes", toJson ([] : List String)),
  ("transfers", toJson ([] : List String)), ("receives", toJson ([] : List String)),
  ("grade", emptyObject), ("relies", toJson ([] : List String)),
  ("guarantees", toJson ([] : List String))]

-- Decoder unit fixtures have no compositions, refinements, or certificates.
private def envelope (enums : List (String × List String)) (system : Json) : Json :=
  Json.mkObj [
    ("schema", toJson "behavior-core-v1"), ("source_path", toJson "unit-test.nmlt"),
    ("source_sha256", toJson (String.ofList (List.replicate 64 '0'))),
    ("enums", Json.mkObj (enums.map fun (name, variants) => (name, toJson variants))),
    ("systems", Json.mkObj [("Test", system)]), ("compositions", emptyObject),
    ("refinements", toJson ([] : List Json))]

private def finiteFixture
    (enums : List (String × List String)) (typeName : String) (initial : EncodedTerm)
    (guards : List EncodedTerm := []) (update : Option EncodedTerm := none) : Json :=
  let updates := update.toList.map fun term => ("current", toJson term.text)
  let updateAst := update.toList.map fun term => ("current", term.ast)
  envelope enums (Json.mkObj [
    ("state", Json.mkObj [("current", Json.mkObj [
      ("type", toJson typeName), ("initial", toJson initial.text),
      ("initial_ast", initial.ast)])]),
    ("capabilities", emptyObject), ("ports", emptyObject),
    ("observe", toJson ["current"]),
    ("actions", Json.mkObj [("tick", Json.mkObj [
      ("direction", toJson "internal"), ("hidden", toJson false),
      ("parameters", toJson ([] : List Json)), ("outputs", toJson ([] : List String)),
      ("resources", emptyResources), ("guards", toJson (guards.map EncodedTerm.text)),
      ("guard_ast", toJson (guards.map EncodedTerm.ast)),
      ("updates", Json.mkObj updates), ("update_ast", Json.mkObj updateAst)])])])

private def accepted (label : String) (fixture : Json) : IO Program := do
  match decodeProgram fixture with
  | .ok program => pure program
  | .error message => throw (IO.userError s!"{label}: {message}")

private def rejected (label expected : String) (fixture : Json) : IO Unit := do
  -- Exercise only rejection at the normative decoder, never theorem closure.
  match decode fixture with
  | .error message =>
      check ((message.splitOn expected).length > 1) s!"{label}: unexpected error: {message}"
  | .ok _ => throw (IO.userError s!"{label}: decoder accepted an invalid finite term")

private def finiteControl (label : String) (fixture : Json) : IO Unit := do
  let program ← accepted label fixture
  let some system := program.systems.head? | throw (IO.userError s!"{label}: no system")
  let indices := List.finRange ((stateSpace program system).length + 1)
  let initial := indices.filter fun state =>
    decide ((toBehavior program system ["tick"] []).init state)
  check (initial.length == 1) s!"{label}: expected exactly one initial state"
  for before in initial do
    check (indices.any fun after =>
      decide ((toBehavior program system ["tick"] []).step before ⟨0, by decide⟩ after))
      s!"{label}: initial transition left the finite state space"
  for field in system.state do
    let some value := evaluateTerm field.initial [] |
      throw (IO.userError s!"{label}: initializer did not evaluate")
    check ((typeDomain program field.typeName).contains value)
      s!"{label}: initializer is outside its declared domain"

private def checkFiniteTerms : IO Unit := do
  let colors := [("Color", ["Blue", "Red"])]
  finiteControl "Bool update" (finiteFixture [] "Bool" (boolTerm false) [] (boolTerm true))
  finiteControl "Unit update" (finiteFixture [] "Unit" unitTerm [] unitTerm)
  finiteControl "enum update" (finiteFixture colors "Color" (enumTerm "Color" "Blue")
    [] (enumTerm "Color" "Red"))
  finiteControl "enum read and equality" (finiteFixture colors "Color" (enumTerm "Color" "Blue")
    [equalTerm (readTerm "Color") (enumTerm "Color" "Blue")] (readTerm "Color"))
  finiteControl "nested Bool initializer" (finiteFixture colors "Bool"
    (notTerm (equalTerm (enumTerm "Color" "Blue") (enumTerm "Color" "Red"))))
  -- Contract declarations do not override the builtin value domains.
  let builtinFacts := [("Bool", ["Fact"]), ("Unit", ["Fact"])]
  finiteControl "builtin contract declarations" (finiteFixture builtinFacts "Bool" (boolTerm false))
  for typeName in ["Bool", "Unit"] do
    rejected s!"{typeName} enum initializer" "enum literal has builtin type"
      (finiteFixture builtinFacts typeName (enumTerm typeName "Fact"))
    let initial := if typeName == "Bool" then boolTerm false else unitTerm
    rejected s!"{typeName} enum update" "enum literal has builtin type"
      (finiteFixture builtinFacts typeName initial [] (enumTerm typeName "Fact"))
    rejected s!"{typeName} equality operand" "enum literal has builtin type"
      (finiteFixture builtinFacts typeName initial
        [equalTerm initial (enumTerm typeName "Fact")])
  rejected "enum under negation" "enum literal has builtin type"
    (finiteFixture builtinFacts "Bool" (notTerm (enumTerm "Bool" "Fact")))
  rejected "enum as equality's inferred operand type" "enum literal has builtin type"
    (finiteFixture builtinFacts "Bool"
      (equalTerm (enumTerm "Unit" "Fact") (enumTerm "Unit" "Fact")))
  -- Exact keys and constructor entries remain authoritative even with delimiters.
  let qualified := [("Color", ["Blue"]), ("Color.Shade", ["Dark"])]
  finiteControl "exact qualified enum" (finiteFixture qualified "Color.Shade"
    (enumTerm "Color.Shade" "Dark"))
  rejected "undeclared type prefix" "outside the finite core"
    (finiteFixture [("Color.Shade", ["Dark"])] "Color" (enumTerm "Color" "Shade.Dark"))
  rejected "constructor belongs to another exact type" "unknown constructor"
    (finiteFixture qualified "Color" (enumTerm "Color" "Shade.Dark"))
  rejected "nested unknown constructor" "unknown constructor"
    (finiteFixture qualified "Bool"
      (equalTerm (enumTerm "Color" "Blue") (enumTerm "Color" "Shade.Dark")))

private def checkCapabilityCoverage : IO Unit := do
  let input := Json.mkObj [
    ("direction", toJson "input"), ("hidden", toJson false),
    ("parameters", toJson [Json.mkObj [("name", toJson "incoming"), ("type", toJson "Once<Unit>")]]),
    ("outputs", toJson ([] : List String)),
    ("resources", Json.mkObj [
      ("requires", toJson ["owned"]), ("consumes", toJson ([] : List String)),
      ("transfers", toJson ([] : List String)), ("receives", toJson ["incoming"]),
      ("grade", emptyObject), ("relies", toJson ([] : List String)),
      ("guarantees", toJson ([] : List String))]),
    ("guards", toJson ([] : List String)), ("guard_ast", toJson ([] : List Json)),
    ("updates", emptyObject), ("update_ast", emptyObject)]
  let fixture := envelope [] (Json.mkObj [
    ("state", emptyObject), ("capabilities", Json.mkObj [("owned", toJson "Once<Unit>")]),
    ("ports", Json.mkObj [("accept", Json.mkObj [
      ("direction", toJson "input"), ("payload", toJson "Once<Unit>")])]),
    ("actions", Json.mkObj [("accept", input)]), ("observe", toJson ([] : List String))])
  let program ← accepted "open input capability" fixture
  let some system := program.systems.head? | throw (IO.userError "missing input system")
  check (capabilityNames program == ["owned", "incoming"])
    "capability universe must preserve ownership order and append input bindings"
  let indices := List.finRange ((capabilityNames program).length + 1)
  for capability in indices do
    let name := nameAt? (capabilityNames program) capability
    check (decide ((toBehavior program system ["accept"] []).owns capability) ==
      (name == some "owned")) "referenced authority must not grant ownership"
    check (decide (((toBehavior program system ["accept"] []).resources ⟨0, by decide⟩).receives capability) ==
      (name == some "incoming")) "decoded receive predicate must retain its named capability"
    check (decide (((toBehavior program system ["accept"] []).resources ⟨0, by decide⟩).requires capability) ==
      (name == some "owned")) "decoded requirement predicate must retain its named capability"
  check ((stateSpace program system).length == 1) "empty-state systems must retain their state"
  -- Cover the semantic constructor's full Profile input independently of ownership.
  let resources : Profile := {
    requires := ["require"], consumes := ["consume"], transfers := ["transfer"],
    receives := ["receive"], grade := [], relies := [], guarantees := [] }
  let action : Action := {
    name := "profile", direction := .internal, hidden := false,
    guards := [], updates := [], resources }
  let profileSystem := { system with capabilities := [], actions := [action] }
  let profileProgram := { program with systems := [profileSystem] }
  check (capabilityNames profileProgram == ["require", "consume", "transfer", "receive"])
    "every Profile capability list must be represented"

def main : IO Unit := do
  checkFiniteTerms
  checkCapabilityCoverage
  IO.println "artifact invariant checks passed"
