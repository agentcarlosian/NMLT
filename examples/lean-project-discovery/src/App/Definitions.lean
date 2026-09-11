module
/- Import discovery uses Lean's parser, including nested comments.
   /- import Missing.Nested -/
   import Missing.Commented
-/
public import Support.Core

public section
namespace App

def value (n : Nat) : Nat := Support.offset n

theorem value_eq (n : Nat) : value n = n := Support.offset_eq n

end App
