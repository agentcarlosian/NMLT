namespace Support
def shift (n : Nat) : Nat := 0 + n
theorem shift_eq (n : Nat) : shift n = n := Nat.zero_add n
end Support
