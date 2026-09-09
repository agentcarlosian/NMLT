//! A closed term grammar, not an arbitrary Lean command/tactic channel.
use crate::Error;

/// Parse and fully parenthesize an Init expression. Identifiers are references,
/// never commands; there are no quotations, macros, tactics, comments or IO.
pub(crate) fn render(source: &str) -> Result<String, Error> {
    if source.is_empty() || source.len() > 4096 || !source.is_ascii() {
        return Err(Error("Lean term requires 1..4096 ASCII bytes".into()));
    }
    let mut tokens = vec![];
    let mut rest = source;
    while !rest.is_empty() {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        let n = if rest.starts_with("->") || rest.starts_with("=>") {
            2
        } else if rest.as_bytes()[0].is_ascii_alphabetic() || rest.starts_with('_') {
            rest.bytes()
                .take_while(|b| b.is_ascii_alphanumeric() || b"_.'".contains(b))
                .count()
        } else if rest.as_bytes()[0].is_ascii_digit() {
            rest.bytes().take_while(u8::is_ascii_digit).count()
        } else if b"():,=+*@".contains(&rest.as_bytes()[0]) {
            1
        } else {
            return Err(Error("unsupported character in Lean term".into()));
        };
        tokens.push(&rest[..n]);
        if rest.as_bytes()[0].is_ascii_digit()
            && rest
                .as_bytes()
                .get(n)
                .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
        {
            return Err(Error("invalid Lean numeral boundary".into()));
        }
        rest = &rest[n..];
        if tokens.len() > 512 {
            return Err(Error("Lean term exceeds 512 tokens".into()));
        }
    }
    let mut parser = Parser { tokens, cursor: 0 };
    let result = parser.expr(0, 0)?;
    if parser.peek().is_some() {
        return Err(Error("unexpected trailing Lean term tokens".into()));
    }
    Ok(result)
}

struct Parser<'a> {
    tokens: Vec<&'a str>,
    cursor: usize,
}
fn identifier(s: &str) -> bool {
    s.len() <= 256
        && s.split('.').all(|part| {
            !part.is_empty()
                && part.as_bytes()[0].is_ascii_alphabetic()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_'".contains(&b))
        })
        && ![
            "by",
            "do",
            "let",
            "match",
            "if",
            "then",
            "else",
            "fun",
            "forall",
            "sorry",
            "admit",
            "where",
            "calc",
            "show",
            "have",
            "suffices",
            "return",
            "theorem",
            "def",
            "axiom",
            "unsafe",
            "opaque",
            "example",
            "set_option",
            "import",
            "open",
            "namespace",
            "end",
            "in",
            "syntax",
            "macro",
        ]
        .contains(&s)
}
impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a str> {
        self.tokens.get(self.cursor).copied()
    }
    fn take(&mut self) -> Result<&'a str, Error> {
        let token = self
            .peek()
            .ok_or_else(|| Error("incomplete Lean term".into()))?;
        self.cursor += 1;
        Ok(token)
    }
    fn expect(&mut self, s: &str) -> Result<(), Error> {
        if self.take()? == s {
            Ok(())
        } else {
            Err(Error(format!("expected {s} in Lean term")))
        }
    }
    fn name(&mut self) -> Result<&'a str, Error> {
        let name = self.take()?;
        if identifier(name) {
            Ok(name)
        } else {
            Err(Error("expected Lean identifier".into()))
        }
    }
    fn atom_start(s: &str) -> bool {
        s == "(" || s == "@" || identifier(s) || s.bytes().all(|b| b.is_ascii_digit())
    }
    fn expr(&mut self, min: u8, depth: usize) -> Result<String, Error> {
        if depth >= 48 {
            return Err(Error("Lean term exceeds depth bound".into()));
        }
        let first = self.take()?;
        let mut left = match first {
            "fun" | "forall" if min == 0 => {
                let parenthesized = self.peek() == Some("(");
                if parenthesized { self.take()?; }
                let name = self.name()?;
                if name.contains('.') { return Err(Error("binder must be unqualified".into())); }
                let ty = if self.peek() == Some(":") {
                    self.take()?;
                    Some(self.expr(0, depth + 1)?)
                } else { None };
                if parenthesized { self.expect(")")?; }
                if first == "forall" && ty.is_none() { return Err(Error("forall binder requires a type".into())); }
                self.expect(if first == "fun" { "=>" } else { "," })?;
                let body = self.expr(0, depth + 1)?;
                let binder = ty.map_or_else(|| name.to_string(), |ty| format!("({name} : {ty})"));
                format!("({first} {binder} {} {body})", if first == "fun" { "=>" } else { "," })
            }
            "(" => {
                let inner = self.expr(0, depth + 1)?;
                let inner = if self.peek() == Some(":") {
                    self.take()?;
                    format!("({inner} : {})", self.expr(0, depth + 1)?)
                } else { inner };
                self.expect(")")?;
                inner
            }
            "@" => format!("(@{})", self.name()?),
            s if identifier(s) => format!("({s})"),
            s if s.len() <= 20 && s.bytes().all(|b| b.is_ascii_digit()) => format!("({s})"),
            _ => return Err(Error("unsupported Lean term; use references, application, fun, forall, equality, arrows or Nat arithmetic".into())),
        };
        while let Some(op) = self.peek() {
            let (precedence, right) = match op {
                "->" => (10, true),
                "=" => (30, false),
                "+" => (50, false),
                "*" => (60, false),
                _ if Self::atom_start(op) => (80, false),
                _ => break,
            };
            if precedence < min {
                break;
            }
            let application = precedence == 80;
            if !application {
                self.take()?;
            }
            let rhs = self.expr(if right { precedence } else { precedence + 1 }, depth + 1)?;
            left = if application {
                format!("({left} {rhs})")
            } else {
                format!("({left} {op} {rhs})")
            };
        }
        Ok(left)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_terms_and_bounds() {
        for input in [
            "forall n : Nat, n + 0 = n",
            "fun (n : Nat) => Nat.add_zero n",
            "And True True",
            "And.intro True.intro True.intro",
            "@Eq.refl Nat 2",
            "True -> True",
        ] {
            assert!(render(input).is_ok(), "{input}");
        }
        for input in [
            "by rfl",
            "sorry",
            "x) #eval IO.println 1 (",
            "fun x => by run_tac x",
            "Nat/-x-/",
            "\"message\"",
            "forall n : Nat, n; x",
            "fun x =>",
            "@by",
            "0x10",
        ] {
            assert!(render(input).is_err(), "{input}");
        }
        assert!(render(&format!("{}True{}", "(".repeat(49), ")".repeat(49))).is_err());
        assert!(render(&"x ".repeat(513)).is_err());
    }
}
