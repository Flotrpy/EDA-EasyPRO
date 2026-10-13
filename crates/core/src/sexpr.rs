//! Minimal S-expression reader for KiCad files.

#[derive(Clone, Debug, PartialEq)]
pub enum Sexp {
    Atom(String),
    List(Vec<Sexp>),
}

impl Sexp {
    /// Head symbol of a list, e.g. `pad` for `(pad "1" smd ...)`.
    pub fn head(&self) -> Option<&str> {
        match self {
            Sexp::List(v) => v.first().and_then(|s| s.atom()),
            _ => None,
        }
    }

    pub fn atom(&self) -> Option<&str> {
        match self {
            Sexp::Atom(s) => Some(s),
            _ => None,
        }
    }

    pub fn items(&self) -> &[Sexp] {
        match self {
            Sexp::List(v) => v,
            _ => &[],
        }
    }

    /// Child lists with the given head.
    pub fn all<'a>(&'a self, head: &'a str) -> impl Iterator<Item = &'a Sexp> + 'a {
        self.items().iter().filter(move |s| s.head() == Some(head))
    }

    pub fn get(&self, head: &str) -> Option<&Sexp> {
        self.all(head).next()
    }

    /// The n-th element after the head as a string.
    pub fn arg(&self, n: usize) -> Option<&str> {
        self.items().get(n + 1).and_then(|s| s.atom())
    }

    pub fn num(&self, n: usize) -> Option<f64> {
        self.arg(n).and_then(|s| s.parse().ok())
    }

    pub fn has_atom(&self, a: &str) -> bool {
        self.items().iter().any(|s| s.atom() == Some(a))
    }
}

pub fn parse(text: &str) -> Result<Sexp, String> {
    let mut chars = text.char_indices().peekable();
    let mut stack: Vec<Vec<Sexp>> = vec![vec![]];
    while let Some((i, c)) = chars.next() {
        match c {
            '(' => stack.push(vec![]),
            ')' => {
                let list = stack.pop().ok_or("unbalanced )")?;
                stack.last_mut().ok_or(format!("unbalanced ) at {i}"))?.push(Sexp::List(list));
            }
            '"' => {
                let mut s = String::new();
                loop {
                    match chars.next() {
                        Some((_, '\\')) => {
                            if let Some((_, e)) = chars.next() {
                                s.push(match e {
                                    'n' => '\n',
                                    other => other,
                                });
                            }
                        }
                        Some((_, '"')) => break,
                        Some((_, ch)) => s.push(ch),
                        None => return Err("unterminated string".into()),
                    }
                }
                stack.last_mut().unwrap().push(Sexp::Atom(s));
            }
            c if c.is_whitespace() => {}
            _ => {
                let mut s = String::from(c);
                while let Some(&(_, n)) = chars.peek() {
                    if n.is_whitespace() || n == '(' || n == ')' {
                        break;
                    }
                    s.push(n);
                    chars.next();
                }
                stack.last_mut().unwrap().push(Sexp::Atom(s));
            }
        }
        if stack.is_empty() {
            return Err("unbalanced".into());
        }
    }
    if stack.len() != 1 {
        return Err("unexpected end of file (missing ')')".into());
    }
    let mut top = stack.pop().unwrap();
    match top.len() {
        1 => Ok(top.remove(0)),
        0 => Err("empty file".into()),
        _ => Ok(Sexp::List(top)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_lists_and_strings() {
        let s = parse(r#"(pad "1" smd (at 1.5 -2) (layers "F.Cu" "F.Mask"))"#).unwrap();
        assert_eq!(s.head(), Some("pad"));
        assert_eq!(s.arg(0), Some("1"));
        assert_eq!(s.get("at").unwrap().num(1), Some(-2.0));
        assert_eq!(s.get("layers").unwrap().items().len(), 3);
        assert!(parse("(a (b)").is_err());
    }
}
