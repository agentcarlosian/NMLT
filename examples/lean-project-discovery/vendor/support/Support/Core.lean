module
public import Init

public section
namespace Support

def offset (n : Nat) : Nat := 0 + n

theorem offset_eq (n : Nat) : offset n = n := Nat.zero_add n

end Support
