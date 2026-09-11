module
public import App.Definitions
public meta import Init

public section
namespace App

-- A selected task statement; an accepted proof cannot depend on this placeholder.
theorem checked_value (n : Nat) : value n = n := by sorry

end App
