# NMLT manifesto

NMLT expands to **New Mathematics, Languages, and Techniques**. It is the
umbrella research program; the **NMLT language** is its first flagship
language. The plural form is intentional: one syntax is not enough. The
program must connect mathematical formalisms, human-facing and core languages,
evidence formats, and repeatable development and verification techniques.

Its Latin companion form is ***Nova Mathematica · Linguae · Technicae***.
`Technicae` keeps techniques—plural—in the T position.

## Thesis

To truly progress, humanity needs new mathematics, new languages, and new
techniques.

This is not a claim that novelty is inherently valuable. It is a claim that our
current programming abstractions poorly represent change, concurrency,
uncertainty, authority, and evidence—the things that increasingly determine
whether complex systems help or harm us.

## Programs are claims about change

A value-oriented language asks what an expression computes. NMLT must also ask:

- which states may follow which other states;
- what must never happen;
- what must eventually happen;
- which observations define externally meaningful behavior;
- which resources, permissions, and trust relationships permit a transition;
- which evidence supports a claim and where that evidence stops.

The central object is therefore not merely a value or instruction sequence. It
is a behavior with an explicit boundary of justification.

## Evidence is part of the result

`proved`, `model_checked`, `tested`, `monitored`, `refuted`, `unknown`, and
`indeterminate` are different outcomes. NMLT must not collapse them into a
green checkmark or a confidence score.

Every positive result carries its assumptions, scope, bounds, engine identity,
source identity, and residual gaps. Every negative result carries a structured
witness when one exists.

## Composition is the real test

A local component may be correct while the composed system is unsafe. NMLT
should make ports, effects, assumptions, guarantees, observations, and trust
flows explicit enough that composition itself becomes a checkable operation.

## Automation proposes; small kernels decide

Humans and AI systems may generate models, invariants, proofs, programs, and
repairs. Acceptance depends on independent, reproducible machinery. When that
machinery cannot decide, the language must preserve the uncertainty.

## The project succeeds if

NMLT helps people discover false assumptions earlier, understand failures more
quickly, connect designs to running systems, and build systems whose evidence is
as composable and inspectable as their code.
