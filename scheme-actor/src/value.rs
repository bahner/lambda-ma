//! ma-scheme value representation (ma-scheme-v1.md §5).
//!
//! Values double as both data and the parsed AST — the parser builds a
//! `Value` tree (symbols, pairs, literals) and the evaluator walks that
//! same tree directly. This is ordinary for a Scheme-family interpreter;
//! it does not contradict "no eval-able code as data" (§5, §18) — that
//! rule is about what a *script* can do at runtime (no `eval`/`include`
//! builtin reachable from ma-scheme itself), not about the host's own
//! internal implementation strategy.

use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use crate::env::Env;

/// A conforming host's core value type (§5): integers, floats, strings,
/// booleans, symbols, proper lists (pairs + the empty list), maps,
/// lambdas, and the opaque `msg` record type. No vector/record type
/// beyond string-keyed maps and `msg`.
#[derive(Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    Symbol(Rc<str>),
    Bool(bool),
    /// The empty list, `'()`.
    Nil,
    /// A cons cell. Proper lists are chains of these ending in `Nil`.
    Pair(Rc<(Value, Value)>),
    /// A deterministic, string-keyed map. Values may be any ordinary
    /// ma-scheme data value that can be CBOR-encoded.
    Map(BTreeMap<String, Value>),
    Lambda(Rc<Lambda>),
    /// A host-provided builtin procedure (§8). The `&'static str` is its
    /// name, used for error messages and `procedure?`/display purposes.
    Builtin(&'static str, BuiltinFn),
    /// The opaque, read-only `msg` record (§4) — not constructible from
    /// ma-scheme itself, only ever handed to a script by the host.
    Msg(Rc<crate::msg::MsgRecord>),
    /// A CID-reference literal (§5): `#/ipfs/<cid>` or `#/ipns/<key>`,
    /// read as a single opaque token, never a string or symbol. Its only
    /// legal use is as the direct, literal argument to `ma-include-ipfs`
    /// (§11.1) — there is no way to construct, coerce, or inspect one from
    /// any other value. The `Rc<str>` holds the reference exactly as
    /// written, including the `#` prefix (e.g. `"#/ipfs/bafy..."`).
    IpfsRef(Rc<str>),
}

pub type BuiltinFn = fn(&[Value]) -> Result<Value, EvalError>;

/// A user-defined procedure created by `lambda` (directly, or via the
/// `(define (name args...) body...)` sugar).
pub struct Lambda {
    pub name: Option<Rc<str>>,
    pub params: Vec<Rc<str>>,
    /// Implicit `begin`: every expression but the last is evaluated for
    /// effect only; the last is in tail position.
    pub body: Vec<Value>,
    pub env: Rc<Env>,
}

/// A single, string-message evaluation error. Kept as a plain string
/// (rather than pulling in a full error-chaining crate) — script errors
/// are reported to callers as `[:error, reason]` text, not backtraces.
#[derive(Debug, Clone)]
pub struct EvalError(pub String);

impl EvalError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for EvalError {}

pub type EvalResult<T> = Result<T, EvalError>;

impl Value {
    pub fn str(s: impl Into<String>) -> Self {
        Value::Str(Rc::from(s.into()))
    }

    pub fn symbol(s: impl Into<String>) -> Self {
        Value::Symbol(Rc::from(s.into()))
    }

    pub fn cons(a: Value, b: Value) -> Self {
        Value::Pair(Rc::new((a, b)))
    }

    /// Build a proper list from a `Vec`, terminated by `Nil`.
    pub fn list(items: Vec<Value>) -> Self {
        items
            .into_iter()
            .rev()
            .fold(Value::Nil, |acc, v| Value::cons(v, acc))
    }

    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    pub fn is_pair(&self) -> bool {
        matches!(self, Value::Pair(_))
    }

    pub fn as_symbol(&self) -> Option<&str> {
        match self {
            Value::Symbol(s) => Some(s),
            _ => None,
        }
    }

    /// Everything except `#f` is truthy (standard Scheme convention).
    pub fn is_truthy(&self) -> bool {
        !matches!(self, Value::Bool(false))
    }

    pub fn car(&self) -> EvalResult<Value> {
        match self {
            Value::Pair(p) => Ok(p.0.clone()),
            _ => Err(EvalError::new(format!("car: not a pair: {self}"))),
        }
    }

    pub fn cdr(&self) -> EvalResult<Value> {
        match self {
            Value::Pair(p) => Ok(p.1.clone()),
            _ => Err(EvalError::new(format!("cdr: not a pair: {self}"))),
        }
    }

    /// Collect a proper list into a `Vec`. Errors on an improper
    /// (dotted-pair-terminated) list.
    pub fn to_vec(&self) -> EvalResult<Vec<Value>> {
        let mut out = Vec::new();
        let mut cur = self.clone();
        loop {
            match cur {
                Value::Nil => break,
                Value::Pair(p) => {
                    out.push(p.0.clone());
                    cur = p.1.clone();
                }
                other => {
                    return Err(EvalError::new(format!(
                        "expected a proper list, found improper tail: {other}"
                    )))
                }
            }
        }
        Ok(out)
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "integer",
            Value::Float(_) => "float",
            Value::Str(_) => "string",
            Value::Symbol(_) => "symbol",
            Value::Bool(_) => "boolean",
            Value::Nil => "nil",
            Value::Pair(_) => "pair",
            Value::Map(_) => "map",
            Value::Lambda(_) => "lambda",
            Value::Builtin(..) => "procedure",
            Value::Msg(_) => "msg",
            Value::IpfsRef(_) => "ipfs-ref",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(x) => write!(f, "{x}"),
            Value::Str(s) => write!(f, "{s:?}"),
            Value::Symbol(s) => write!(f, "{s}"),
            Value::Bool(true) => write!(f, "#t"),
            Value::Bool(false) => write!(f, "#f"),
            Value::Nil => write!(f, "()"),
            Value::Pair(_) => {
                write!(f, "(")?;
                let mut cur = self.clone();
                let mut first = true;
                loop {
                    match cur {
                        Value::Pair(p) => {
                            if !first {
                                write!(f, " ")?;
                            }
                            write!(f, "{}", p.0)?;
                            first = false;
                            cur = p.1.clone();
                        }
                        Value::Nil => break,
                        other => {
                            write!(f, " . {other}")?;
                            break;
                        }
                    }
                }
                write!(f, ")")
            }
            Value::Map(m) => {
                write!(f, "#<map (")?;
                for (idx, (key, value)) in m.iter().enumerate() {
                    if idx > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "({key:?} . {value})")?;
                }
                write!(f, ")>")
            }
            Value::Lambda(l) => match &l.name {
                Some(n) => write!(f, "#<lambda:{n}>"),
                None => write!(f, "#<lambda>"),
            },
            Value::Builtin(name, _) => write!(f, "#<builtin:{name}>"),
            Value::Msg(_) => write!(f, "#<msg>"),
            Value::IpfsRef(r) => write!(f, "{r}"),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

/// Structural equality (`equal?`, §8) — deep comparison for pairs/lists,
/// value comparison for scalars. Lambdas/builtins are only equal by
/// reference-ish identity (never equal to a different procedure value);
/// not meaningfully comparable otherwise.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) | (Value::Float(b), Value::Int(a)) => {
                (*a as f64) == *b
            }
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Symbol(a), Value::Symbol(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Pair(a), Value::Pair(b)) => a.0 == b.0 && a.1 == b.1,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::IpfsRef(a), Value::IpfsRef(b)) => a == b,
            _ => false,
        }
    }
}
