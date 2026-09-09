import NMLT.Artifact.SemanticClosure

namespace NMLT.Artifact.SafetyPredicate
open Lean (Json)
open NMLT.Artifact.BehaviorCore NMLT.Artifact.SemanticClosure

inductive Predicate where
  | boolean (term : Term)
  | negation (value : Predicate)
  | conjunction (left right : Predicate)
  | disjunction (left right : Predicate)
  | implication (left right : Predicate)
deriving Repr, BEq

def evaluate : Predicate → Valuation → Bool
  | .boolean term, state => evaluateTerm term state == some (.bool true)
  | .negation value, state => !(evaluate value state)
  | .conjunction left right, state => evaluate left state && evaluate right state
  | .disjunction left right, state => evaluate left state || evaluate right state
  | .implication left right, state => !(evaluate left state) || evaluate right state

def keys (json : Json) (expected : List String) : Except String Unit := do
  let actual := (← json.getObj?).toList.map Prod.fst
  unless actual.length == expected.length && actual.all expected.contains do
    throw s!"safety: unexpected object fields {actual}"

def stringAt (json : Json) (key : String) : Except String String := do
  (← json.getObjVal? key).getStr?

private def termType : Term → String
  | .bool _ | .not _ | .equal _ _ => "Bool"
  | .unit => "Unit"
  | .enumeration name _ | .read name _ => name

private def checkedRead (system : System) (name : String) : Except String Term := do
  let some field := system.state.find? (fun f => f.name == name) |
    throw s!"safety: unknown state field '{name}'"
  pure (.read field.typeName name)

private def checkedEnum (program : Program) (typeName name : String) : Except String Term := do
  let some (_, variants) := program.enums.find? (fun e => e.1 == typeName) |
    throw s!"safety: unknown enum '{typeName}'"
  unless variants.contains name do throw s!"safety: unknown constructor '{typeName}.{name}'"
  pure (.enumeration typeName name)

private def decodeTerm (program : Program) (system : System) : Nat → Json → Except String Term
  | 0, _ => throw "safety: term depth exceeds bound"
  | fuel + 1, json => do
    let kind ← stringAt json "kind"
    let encodedType ← stringAt json "type"
    let term ← match kind with
      | "bool" =>
        keys json ["kind", "type", "value"]
        pure (.bool (← (← json.getObjVal? "value").getBool?))
      | "unit" => keys json ["kind", "type"]; pure .unit
      | "read" =>
        keys json ["kind", "type", "field"]
        checkedRead system (← stringAt json "field")
      | "enum" =>
        keys json ["kind", "type", "constructor"]
        checkedEnum program encodedType (← stringAt json "constructor")
      | "not" =>
        keys json ["kind", "type", "value"]
        let value ← decodeTerm program system fuel (← json.getObjVal? "value")
        unless termType value == "Bool" do throw "safety: negation needs Bool"
        pure (.not value)
      | "equal" =>
        keys json ["kind", "type", "left", "right"]
        let left ← decodeTerm program system fuel (← json.getObjVal? "left")
        let right ← decodeTerm program system fuel (← json.getObjVal? "right")
        unless termType left == termType right do throw "safety: equality operand types differ"
        pure (.equal left right)
      | _ => throw s!"safety: unknown finite term '{kind}'"
    unless termType term == encodedType do throw "safety: encoded term type differs from declaration"
    pure term

def decode (program : Program) (system : System) : Nat → Json → Except String Predicate
  | 0, _ => throw "safety: predicate depth exceeds bound"
  | fuel + 1, json => do
    match ← stringAt json "kind" with
    | "boolean" =>
      keys json ["kind", "term"]
      let term ← decodeTerm program system 64 (← json.getObjVal? "term")
      unless termType term == "Bool" do throw "safety: predicate must have type Bool"
      pure (.boolean term)
    | "not" =>
      keys json ["kind", "value"]
      pure (.negation (← decode program system fuel (← json.getObjVal? "value")))
    | "and" | "or" | "implies" =>
      keys json ["kind", "left", "right"]
      let left ← decode program system fuel (← json.getObjVal? "left")
      let right ← decode program system fuel (← json.getObjVal? "right")
      match ← stringAt json "kind" with
      | "and" => pure (.conjunction left right)
      | "or" => pure (.disjunction left right)
      | _ => pure (.implication left right)
    | kind => throw s!"safety: unknown predicate '{kind}'"

private def identifierStart (c : Char) :=
  ('a' ≤ c && c ≤ 'z') || ('A' ≤ c && c ≤ 'Z') || c == '_'
private def identifierRest (c : Char) := identifierStart c || c.isDigit

private def skipBlock : Nat → List Char → Except String (List Char)
  | 0, _ => throw "safety: comment exceeds bound"
  | _, [] => throw "safety: unterminated block comment"
  | _ + 1, '*' :: '/' :: rest => pure rest
  | fuel + 1, _ :: rest => skipBlock fuel rest

private def lex : Nat → List Char → Except String (List String)
  | 0, _ => throw "safety: lexical bound exceeded"
  | _ + 1, [] => pure []
  | fuel + 1, '/' :: '/' :: rest => lex fuel (rest.dropWhile (· != '\n'))
  | fuel + 1, '/' :: '*' :: rest => do lex fuel (← skipBlock 4096 rest)
  | fuel + 1, '=' :: '=' :: rest => return "==" :: (← lex fuel rest)
  | fuel + 1, '!' :: '=' :: rest => return "!=" :: (← lex fuel rest)
  | fuel + 1, '&' :: '&' :: rest => return "&&" :: (← lex fuel rest)
  | fuel + 1, '|' :: '|' :: rest => return "||" :: (← lex fuel rest)
  | fuel + 1, c :: rest => do
    if c.isWhitespace then lex fuel rest
    else if identifierStart c then
      let (suffix, tail) := rest.span identifierRest
      return String.ofList (c :: suffix) :: (← lex fuel tail)
    else if ['(', ')', '.', '!', '='].contains c then
      return String.singleton c :: (← lex fuel rest)
    else throw s!"safety: unsupported character '{c}'"

private abbrev Parser := StateT (List String) (Except String)
private def takeToken (token : String) : Parser Bool := do
  match ← get with
  | first :: rest =>
    if first == token then set rest; pure true else pure false
  | [] => pure false
private def expectToken (token : String) : Parser Unit := do
  unless ← takeToken token do throw s!"safety: expected '{token}'"
private def popToken : Parser String := do
  let first :: rest ← get | throw "safety: missing value"
  set rest
  pure first
private def qualifiedRest : Nat → String → Parser String
  | 0, _ => throw "safety: qualified name exceeds bound"
  | fuel + 1, name => do
    if ← takeToken "." then qualifiedRest fuel (name ++ "." ++ (← popToken))
    else pure name

private def parseValue (program : Program) (system : System) : Parser Term := do
  let first ← popToken
  match first with
  | "true" => pure (.bool true)
  | "false" => pure (.bool false)
  | "unit" => pure .unit
  | _ =>
    let name ← qualifiedRest 256 first
    if system.state.any (fun field => field.name == name) then
      liftM (checkedRead system name)
    else
      let constructor :: qualifier := (name.splitOn ".").reverse |
        throw "safety: missing qualified enum"
      liftM (checkedEnum program (String.intercalate "." qualifier.reverse) constructor)

mutual
private partial def parseImplies (program : Program) (system : System) (fuel : Nat) : Parser Predicate := do
  let fuel + 1 := fuel | throw "safety: parse depth exceeded"
  let left ← parseOr program system fuel
  if ← takeToken "implies" then pure (.implication left (← parseImplies program system fuel))
  else pure left

private partial def parseOr (program : Program) (system : System) (fuel : Nat) : Parser Predicate := do
  let left ← parseAnd program system fuel
  parseOrRest program system fuel left

private partial def parseOrRest (program : Program) (system : System) (fuel : Nat) (left : Predicate) : Parser Predicate := do
  let fuel + 1 := fuel | throw "safety: parse depth exceeded"
  let first ← takeToken "or"
  let second ← if first then pure true else takeToken "||"
  if second then parseOrRest program system fuel (.disjunction left (← parseAnd program system fuel))
  else pure left

private partial def parseAnd (program : Program) (system : System) (fuel : Nat) : Parser Predicate := do
  let left ← parseNot program system fuel
  parseAndRest program system fuel left

private partial def parseAndRest (program : Program) (system : System) (fuel : Nat) (left : Predicate) : Parser Predicate := do
  let fuel + 1 := fuel | throw "safety: parse depth exceeded"
  let first ← takeToken "and"
  let second ← if first then pure true else takeToken "&&"
  if second then parseAndRest program system fuel (.conjunction left (← parseNot program system fuel))
  else pure left

private partial def parseNot (program : Program) (system : System) (fuel : Nat) : Parser Predicate := do
  let fuel + 1 := fuel | throw "safety: parse depth exceeded"
  let first ← takeToken "not"
  let second ← if first then pure true else takeToken "!"
  if second then pure (.negation (← parseNot program system fuel))
  else if ← takeToken "(" then
    let result ← parseImplies program system fuel
    expectToken ")"
    pure result
  else
    let left ← parseValue program system
    let equal ← takeToken "=="
    let different ← if equal then pure false else takeToken "!="
    if equal || different then
      let right ← parseValue program system
      unless termType left == termType right do throw "safety: equality operand types differ"
      let result := Predicate.boolean (.equal left right)
      pure (if different then .negation result else result)
    else
      unless termType left == "Bool" do throw "safety: predicate must have type Bool"
      pure (.boolean left)
end

def parse (program : Program) (system : System) (source : String) : Except String Predicate := do
  unless source.utf8ByteSize ≤ 4096 do throw "safety: predicate exceeds 4096 bytes"
  let tokens ← lex 4097 source.toList
  unless tokens.length ≤ 256 do throw "safety: predicate exceeds 256 tokens"
  let action : Parser Predicate := do
    expectToken "always"
    expectToken "("
    let result ← parseImplies program system 256
    expectToken ")"
    unless (← get).isEmpty do throw "safety: unsupported trailing predicate syntax"
    pure result
  return (← action.run tokens).1

def checkDeclaration (name expression declaration : String) : Except String Unit := do
  unless declaration.utf8ByteSize ≤ 8192 do throw "safety: declaration exceeds bound"
  let actual ← lex 8193 declaration.toList
  let expected ← lex 4097 expression.toList
  unless actual == ["safety", name, "="] ++ expected do
    throw "safety: property name/expression does not match the source declaration"

end NMLT.Artifact.SafetyPredicate
