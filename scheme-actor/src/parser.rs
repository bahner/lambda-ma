//! S-expression lexer/parser (ma-scheme-v1.md §5).
//!
//! Standard S-expression syntax: parenthesised forms, `;` line comments,
//! `'x` sugar for `(quote x)`, standard numeric/string literal syntax.

use std::rc::Rc;

use crate::value::{EvalError, EvalResult, Value};

pub struct Parser<'a> {
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    src: &'a str,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            chars: src.char_indices().peekable(),
            src,
        }
    }

    /// Parse every top-level expression in the source, in order.
    pub fn parse_all(src: &'a str) -> EvalResult<Vec<Value>> {
        let mut p = Parser::new(src);
        let mut out = Vec::new();
        loop {
            p.skip_whitespace_and_comments();
            if p.peek_char().is_none() {
                break;
            }
            out.push(p.parse_expr()?);
        }
        Ok(out)
    }

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|&(_, c)| c)
    }

    fn next_char(&mut self) -> Option<char> {
        self.chars.next().map(|(_, c)| c)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek_char() {
                Some(c) if c.is_whitespace() => {
                    self.next_char();
                }
                Some(';') => {
                    while let Some(c) = self.peek_char() {
                        if c == '\n' {
                            break;
                        }
                        self.next_char();
                    }
                }
                _ => break,
            }
        }
    }

    fn parse_expr(&mut self) -> EvalResult<Value> {
        self.skip_whitespace_and_comments();
        match self.peek_char() {
            None => Err(EvalError::new("unexpected end of input")),
            Some('(') => self.parse_list(),
            Some(')') => Err(EvalError::new("unexpected ')'")),
            Some('\'') => {
                self.next_char();
                let quoted = self.parse_expr()?;
                Ok(Value::list(vec![Value::symbol("quote"), quoted]))
            }
            Some('"') => self.parse_string(),
            _ => self.parse_atom(),
        }
    }

    fn parse_list(&mut self) -> EvalResult<Value> {
        self.next_char(); // consume '('
        let mut items = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            match self.peek_char() {
                None => return Err(EvalError::new("unexpected end of input in list")),
                Some(')') => {
                    self.next_char();
                    break;
                }
                _ => items.push(self.parse_expr()?),
            }
        }
        Ok(Value::list(items))
    }

    fn parse_string(&mut self) -> EvalResult<Value> {
        self.next_char(); // consume opening '"'
        let mut out = String::new();
        loop {
            match self.next_char() {
                None => return Err(EvalError::new("unterminated string literal")),
                Some('"') => break,
                Some('\\') => match self.next_char() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some(other) => out.push(other),
                    None => return Err(EvalError::new("unterminated string literal")),
                },
                Some(c) => out.push(c),
            }
        }
        Ok(Value::str(out))
    }

    /// An atom is a run of characters up to the next delimiter
    /// (whitespace, parenthesis, or `;`). Classified afterward as a
    /// boolean, integer, float, or symbol.
    fn parse_atom(&mut self) -> EvalResult<Value> {
        let start = self.chars.peek().map(|&(i, _)| i).unwrap_or(self.src.len());
        let mut end = start;
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() || c == '(' || c == ')' || c == ';' || c == '"' || c == '\'' {
                break;
            }
            self.next_char();
            end = self.chars.peek().map(|&(i, _)| i).unwrap_or(self.src.len());
        }
        let text = &self.src[start..end];
        if text.is_empty() {
            return Err(EvalError::new(format!(
                "unexpected character: {:?}",
                self.peek_char()
            )));
        }
        classify_atom(text)
    }
}

fn classify_atom(text: &str) -> EvalResult<Value> {
    match text {
        "#t" => return Ok(Value::Bool(true)),
        "#f" => return Ok(Value::Bool(false)),
        _ => {}
    }
    if let Some(rest) = text.strip_prefix('#') {
        // CID-reference literal (§5): #/ipfs/<cid> or #/ipns/<key>. Read
        // as a single opaque token — never a string, never a symbol — its
        // only legal use is as ma-include-ipfs's literal argument (§11.1).
        let is_valid = (rest
            .strip_prefix("/ipfs/")
            .is_some_and(|cid| !cid.is_empty()))
            || (rest
                .strip_prefix("/ipns/")
                .is_some_and(|key| !key.is_empty()));
        if !is_valid {
            return Err(EvalError::new(format!(
                "malformed CID-reference literal {text:?}: expected #/ipfs/<cid> or #/ipns/<key>"
            )));
        }
        return Ok(Value::IpfsRef(Rc::from(text)));
    }
    if text.starts_with('#') {
        return Err(EvalError::new(format!("unknown # syntax: {text:?}")));
    }
    if let Ok(i) = text.parse::<i64>() {
        return Ok(Value::Int(i));
    }
    if let Ok(x) = text.parse::<f64>() {
        // Reject things like a bare "." or symbols float would wrongly
        // accept — f64::parse is fairly strict already, this is just a
        // guard against accepting non-numeric-looking symbols.
        if text
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || c == '-' || c == '+' || c == '.')
        {
            return Ok(Value::Float(x));
        }
    }
    Ok(Value::Symbol(Rc::from(text)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_one(src: &str) -> Value {
        let mut all = Parser::parse_all(src).unwrap();
        assert_eq!(all.len(), 1, "expected exactly one top-level expression");
        all.remove(0)
    }

    #[test]
    fn parses_integers_and_floats() {
        assert_eq!(parse_one("42"), Value::Int(42));
        assert_eq!(parse_one("-7"), Value::Int(-7));
        matches!(parse_one("3.14"), Value::Float(_));
    }

    #[test]
    fn parses_booleans() {
        assert_eq!(parse_one("#t"), Value::Bool(true));
        assert_eq!(parse_one("#f"), Value::Bool(false));
    }

    #[test]
    fn parses_strings_with_escapes() {
        assert_eq!(parse_one(r#""hello\nworld""#), Value::str("hello\nworld"));
    }

    #[test]
    fn parses_symbols_including_colon_atoms() {
        assert_eq!(parse_one("foo"), Value::symbol("foo"));
        assert_eq!(parse_one(":ok"), Value::symbol(":ok"));
        assert_eq!(parse_one("set!"), Value::symbol("set!"));
    }

    #[test]
    fn parses_quote_sugar() {
        let parsed = parse_one("'(1 2 3)");
        let expected = Value::list(vec![
            Value::symbol("quote"),
            Value::list(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
        ]);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parses_nested_lists() {
        let parsed = parse_one("(if (= 1 1) :yes :no)");
        let expected = Value::list(vec![
            Value::symbol("if"),
            Value::list(vec![Value::symbol("="), Value::Int(1), Value::Int(1)]),
            Value::symbol(":yes"),
            Value::symbol(":no"),
        ]);
        assert_eq!(parsed, expected);
    }

    #[test]
    fn parses_multiple_top_level_forms() {
        let all = Parser::parse_all("(define x 1) (define y 2)").unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn skips_comments() {
        let all = Parser::parse_all("; a comment\n(define x 1) ; trailing\n").unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn empty_list_parses_as_nil() {
        assert_eq!(parse_one("()"), Value::Nil);
    }

    #[test]
    fn parses_ipfs_cid_reference_literal() {
        assert_eq!(
            parse_one("#/ipfs/bafybeihelloworld"),
            Value::IpfsRef(Rc::from("#/ipfs/bafybeihelloworld"))
        );
    }

    #[test]
    fn parses_ipns_key_reference_literal() {
        assert_eq!(
            parse_one("#/ipns/k51qzi5uqu5dhx5"),
            Value::IpfsRef(Rc::from("#/ipns/k51qzi5uqu5dhx5"))
        );
    }

    #[test]
    fn ipfs_reference_literal_is_not_a_string_or_symbol() {
        let v = parse_one("#/ipfs/bafybei");
        assert!(!matches!(v, Value::Str(_)));
        assert!(!matches!(v, Value::Symbol(_)));
    }

    #[test]
    fn rejects_malformed_hash_slash_reference() {
        assert!(Parser::parse_all("#/ipfs/").is_err());
        assert!(Parser::parse_all("#/other/thing").is_err());
        assert!(Parser::parse_all("#nope").is_err());
        assert!(Parser::parse_all("#!/ipfs/bafybei").is_err());
    }

    #[test]
    fn ma_include_ipfs_call_parses_with_literal_argument() {
        let parsed = parse_one("(ma-include-ipfs #/ipfs/bafybei)");
        let expected = Value::list(vec![
            Value::symbol("ma-include-ipfs"),
            Value::IpfsRef(Rc::from("#/ipfs/bafybei")),
        ]);
        assert_eq!(parsed, expected);
    }
}
