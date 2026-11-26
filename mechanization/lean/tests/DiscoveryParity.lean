import NMLT.Discovery.FiniteConstructions

def main : IO Unit := do
  for (size, step, valid, iso) in NMLT.Discovery.FiniteConstructions.rows do
    IO.println s!"{size}|{step}|{valid}|{iso}"
