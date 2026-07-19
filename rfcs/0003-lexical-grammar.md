# RFC 0003: Lexical grammar v1

- Status: Accepted
- Authors: NMLT project
- Created: 2026-07-18

## Summary

Define a deliberately small UTF-8 lexical grammar and require a lossless token
stream: every source byte belongs to exactly one ordered token, including
comments, whitespace, malformed strings, and unknown punctuation.

## Grammar

```text
source          ::= token*
identifier      ::= [A-Za-z_][A-Za-z0-9_]*
integer         ::= [0-9][0-9_]*
whitespace      ::= (U+0009 | U+000A | U+000D | U+0020)+
line-comment    ::= "//" <bytes through but excluding line ending>
block-comment   ::= "/*" <non-nested bytes> "*/"
string          ::= '"' string-item* '"'
string-item     ::= <UTF-8 scalar except quote, backslash, CR, LF>
                  | "\\" <one UTF-8 scalar>
delimiter       ::= "{" | "}" | "(" | ")" | "[" | "]"
punctuation     ::= <maximal run of remaining ASCII punctuation>
unknown         ::= <one remaining Unicode scalar>
```

The input API accepts a Rust `str`, so UTF-8 validity is established before
lexing. Identifiers are ASCII in v1. Non-ASCII text is permitted in comments
and strings; a non-ASCII scalar elsewhere is preserved as `unknown` for a
later diagnostic. Keywords are identifiers classified by the parser.

No Unicode normalization or line-ending normalization occurs. Block comments
do not nest. A backslash escapes exactly the next scalar at the lexical layer;
escape validity belongs to string lowering.

## Lossless contract

For tokens `t0..tn` and source byte length `L`:

```text
t0.start = 0
tk.end = t(k+1).start
tn.end = L
source == concat(source[token.span] for token in tokens)
```

An empty source has an empty token list. An unterminated construct emits one
error token covering its remaining bytes and a stable diagnostic. The lexer
must never drop malformed input to improve recovery.

## Diagnostics

- `NMLT1001`: unterminated block comment.
- `NMLT1002`: unterminated string literal.
- Parser delimiter errors retain `NMLT0002` during migration.

Spans are half-open UTF-8 byte ranges. Line and column rendering is derived
from source bytes and is not part of identity.

## Negative controls

- `system` inside a comment or string never becomes a declaration keyword.
- A Unicode scalar outside a string/comment remains in the token stream.
- Unterminated comments and strings retain their complete suffix.
- CRLF round-trips byte-for-byte and therefore has a different source ID from
  LF input.
- Adjacent operators and delimiters have no unowned byte gaps.

## Compatibility and implementation

This grammar is pre-alpha but versioned. Phase 1 implements the flat token
stream first, then constructs the green tree selected by ADR 0002. Changing
identifier Unicode policy, comment nesting, or token boundaries requires an
RFC because it affects formatting, diagnostics, source maps, and macros.
