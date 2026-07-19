//! Lexically-scoped environments (ma-scheme-v1.md §7).
//!
//! `define` introduces a binding in the *current* environment; `set!`
//! mutates an existing binding wherever it is found in the lexical chain
//! (an error if the name is unbound anywhere) — this is the ordinary
//! Scheme distinction, and per §7 `set!` never touches persistent entity
//! state (that is exclusively the state-primitives' job, §9).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::value::{EvalError, EvalResult, Value};

pub struct Env {
    vars: RefCell<HashMap<Rc<str>, Value>>,
    parent: Option<Rc<Env>>,
}

impl Env {
    pub fn new_root() -> Rc<Env> {
        Rc::new(Env {
            vars: RefCell::new(HashMap::new()),
            parent: None,
        })
    }

    pub fn new_child(parent: &Rc<Env>) -> Rc<Env> {
        Rc::new(Env {
            vars: RefCell::new(HashMap::new()),
            parent: Some(Rc::clone(parent)),
        })
    }

    /// `define` — introduces or overwrites a binding in *this* frame only.
    pub fn define(&self, name: Rc<str>, value: Value) {
        self.vars.borrow_mut().insert(name, value);
    }

    /// Look up a binding, searching outward through parent frames.
    pub fn lookup(&self, name: &str) -> EvalResult<Value> {
        if let Some(v) = self.vars.borrow().get(name) {
            return Ok(v.clone());
        }
        match &self.parent {
            Some(p) => p.lookup(name),
            None => Err(EvalError::new(format!("unbound variable: {name}"))),
        }
    }

    /// `set!` — mutates the nearest enclosing binding for `name`. Errors
    /// if `name` is not bound anywhere in the lexical chain (there is
    /// nothing to mutate).
    pub fn set(&self, name: &str, value: Value) -> EvalResult<()> {
        if self.vars.borrow().contains_key(name) {
            self.vars.borrow_mut().insert(Rc::from(name), value);
            return Ok(());
        }
        match &self.parent {
            Some(p) => p.set(name, value),
            None => Err(EvalError::new(format!("set!: unbound variable: {name}"))),
        }
    }
}
