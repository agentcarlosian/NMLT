import NMLT.Evaluation.Discovery

def main : IO Unit := do
  for (size, step, valid, iso) in NMLT.Evaluation.Discovery.largerRows do
    IO.println s!"{size}|{step}|{valid}|{iso}"
