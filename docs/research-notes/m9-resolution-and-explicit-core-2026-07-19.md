# M9 resolution and explicit-core research note

Date: 2026-07-19  
Scope: M9-003b all-reference resolution and M9-004 explicit typed core

## Question

How should NMLT represent reference resolution, local binders, typed
elaboration insertions, and system-indexed temporal terms so that the result is
deterministic, replayable, and suitable for a later small kernel?

## Archive search

The local research archive was queried for `scope graphs`, `locally nameless`,
`typed intermediate language`, and `bidirectional typing`, plus longer
NMLT-specific combinations. The long lexical queries returned no matches; the
short queries returned weak or mostly unrelated entries. This is a limitation
of the archive's lexical coverage, not evidence that the design space is novel.
Current primary literature was therefore used to close the evidence gap.

## Current primary sources and implications

- [Stack graphs: Name resolution at scale](https://arxiv.org/abs/2211.01224)
  treats name resolution as a graph problem with paths that can be computed and
  checked independently. NMLT's first fragment has closed direct imports and no
  re-export, so a general graph framework would be excess machinery; the useful
  invariant is retained as an explicit origin-to-target map plus deterministic
  candidate replay.
- [Well-Scoped Locally Nameless Representation of Syntax](https://arxiv.org/abs/2605.08990)
  emphasizes representing local binding so ill-scoped terms are difficult to
  admit. NMLT uses owner-derived `LocalId`s and rejects shadowing in this narrow
  fragment. This is not a claim that NMLT implements that paper's full
  representation or metatheory.
- [Bidirectional Typing](https://arxiv.org/abs/1908.05839) supports separating
  synthesis from checking and making conversions explicit. Accordingly,
  `nmlt-ir` has no implicit `Nat`/`Int` compatibility: checked literal choice
  belongs to elaboration, while `IntFromNat` is an explicit core node.
- [Typed Closure Conversion](https://arxiv.org/abs/1808.04006) is evidence for
  making type preservation visible across an intermediate-language boundary,
  rather than erasing types and attempting to reconstruct them later. NMLT's
  core therefore annotates every node and makes inserted nodes independently
  identifiable while retaining their HIR origin.
- [The Trusted Computing Base of the CompCert Verified Compiler](https://arxiv.org/abs/2201.10280)
  shows why a verified or checked compiler claim must enumerate the practical
  trusted boundary, not only its central theorem/checker. NMLT keeps the
  resolver trusted even after candidate replay because both currently share a
  crate and lookup model; structural core validation likewise does not claim
  HIR-to-core correspondence.

## Accepted design consequences

1. Every textual reference-shaped HIR node has exactly one canonical
   `ResolutionMap` entry, and every entry is replayed against exact source
   spelling and the closed import candidates.
2. Action binders receive stable owner/node-derived `LocalId`s; locals never
   masquerade as global definitions.
3. Source/HIR `NodeId` and elaborated `CoreNodeId` are separate. A core node
   binds its HIR origin plus a bounded canonical insertion path, permitting
   explicit state-predicate/coercion nodes without identity collision.
4. Every core term carries an explicit type. State and temporal propositions
   carry a system `DefId`, and temporal operations are dedicated constructors,
   never ordinary calls returning `Bool`.
5. `CoreProgram::new` validates only structural well-formedness. Faithful
   translation requires the M9-005 derivation and M9-006 independent kernel;
   no result from M9-003b or M9-004 alone is a `CheckedProgram`.
