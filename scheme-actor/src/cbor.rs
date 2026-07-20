//! CBOR <-> ma-scheme value mapping (ma-scheme-v1.md §6).
//!
//! This MUST be the mapping every conforming host uses, so a script sees
//! identical data shapes regardless of host implementation language.

use ciborium::value::Integer;
use ciborium::Value as Cbor;
use std::collections::BTreeMap;

use crate::value::{EvalError, EvalResult, Value};

pub(crate) fn integer_to_i64(value: Integer, context: &str) -> EvalResult<i64> {
    i64::try_from(value).map_err(|_| {
        EvalError::new(format!(
            "{context}: integer is outside the supported i64 range"
        ))
    })
}

/// Decode CBOR bytes into a `Value` per the §6 table.
pub fn decode(bytes: &[u8]) -> EvalResult<Value> {
    let cbor: Cbor = ciborium::de::from_reader(bytes)
        .map_err(|e| EvalError::new(format!("CBOR decode error: {e}")))?;
    decode_cbor_value(&cbor)
}

/// Encode a `Value` to CBOR bytes using the inverse of the §6 mapping.
/// Lambdas, builtins, and `msg` records have no CBOR representation and
/// are rejected (matching §9: state values "may be any ma-scheme value
/// the host can CBOR-encode ... no lambdas, no msg").
pub fn encode(value: &Value) -> EvalResult<Vec<u8>> {
    let cbor = encode_cbor_value(value)?;
    let mut out = Vec::new();
    ciborium::ser::into_writer(&cbor, &mut out)
        .map_err(|e| EvalError::new(format!("CBOR encode error: {e}")))?;
    Ok(out)
}

/// Decode a single already-parsed `ciborium::Value` per the §6 table.
/// Exposed (not just the byte-oriented `decode`) so callers composing a
/// larger CBOR structure themselves (e.g. the props state map, §9) can
/// convert individual entries without a second serialise/deserialise pass.
pub fn decode_cbor_value(cbor: &Cbor) -> EvalResult<Value> {
    match cbor {
        Cbor::Text(s) => {
            if let Some(sym) = s.strip_prefix(':') {
                Ok(Value::symbol(format!(":{sym}")))
            } else {
                Ok(Value::str(s.clone()))
            }
        }
        Cbor::Integer(i) => Ok(Value::Int(integer_to_i64(*i, "CBOR integer")?)),
        Cbor::Float(x) => Ok(Value::Float(*x)),
        Cbor::Bool(b) => Ok(Value::Bool(*b)),
        Cbor::Null => Ok(Value::Nil),
        Cbor::Array(items) => {
            let values = items
                .iter()
                .map(decode_cbor_value)
                .collect::<EvalResult<Vec<_>>>()?;
            Ok(Value::list(values))
        }
        Cbor::Map(entries) => {
            let mut map = BTreeMap::new();
            for (k, v) in entries {
                let Cbor::Text(key) = k else {
                    return Err(EvalError::new(
                        "CBOR map keys must be text strings for ma-scheme maps (§6)",
                    ));
                };
                map.insert(key.clone(), decode_cbor_value(v)?);
            }
            Ok(Value::Map(map))
        }
        other => Err(EvalError::new(format!(
            "unsupported CBOR value for ma-scheme (§6): {other:?}"
        ))),
    }
}

/// Encode a single `Value` to an already-parsed `ciborium::Value`, the
/// inverse of [`decode_cbor_value`]. See [`encode`] for the byte-oriented
/// entry point.
pub fn encode_cbor_value(value: &Value) -> EvalResult<Cbor> {
    match value {
        Value::Int(i) => Ok(Cbor::Integer((*i).into())),
        Value::Float(x) => Ok(Cbor::Float(*x)),
        Value::Bool(b) => Ok(Cbor::Bool(*b)),
        Value::Nil => Ok(Cbor::Null),
        Value::Str(s) => Ok(Cbor::Text(s.to_string())),
        Value::Symbol(s) if s.starts_with(':') => Ok(Cbor::Text(s.to_string())),
        Value::Symbol(_) => Err(EvalError::new(
            "cannot CBOR-encode a non-atom symbol without changing its type",
        )),
        Value::Pair(_) => {
            let items = value.to_vec()?;
            let cbor_items = items
                .iter()
                .map(encode_cbor_value)
                .collect::<EvalResult<Vec<_>>>()?;
            Ok(Cbor::Array(cbor_items))
        }
        Value::Map(map) => {
            let entries = map
                .iter()
                .map(|(k, v)| Ok((Cbor::Text(k.clone()), encode_cbor_value(v)?)))
                .collect::<EvalResult<Vec<_>>>()?;
            Ok(Cbor::Map(entries))
        }
        Value::Lambda(_) | Value::Builtin(..) => Err(EvalError::new(
            "cannot CBOR-encode a procedure (no lambdas in state/messages, §9)",
        )),
        Value::Msg(_) => Err(EvalError::new(
            "cannot CBOR-encode a msg record (no msg in state/messages, §9)",
        )),
        Value::IpfsRef(_) => Err(EvalError::new(
            "cannot CBOR-encode a CID-reference literal (§5 — only legal use is as ma-include-ipfs's literal argument, §11.1)",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip_cbor(cbor: &Cbor) -> Value {
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(cbor, &mut bytes).unwrap();
        decode(&bytes).unwrap()
    }

    #[test]
    fn text_starting_with_colon_becomes_symbol() {
        let v = roundtrip_cbor(&Cbor::Text(":ping".to_string()));
        assert_eq!(v, Value::symbol(":ping"));
    }

    #[test]
    fn text_not_starting_with_colon_becomes_string() {
        let v = roundtrip_cbor(&Cbor::Text("hello".to_string()));
        assert_eq!(v, Value::str("hello"));
    }

    #[test]
    fn integer_maps_to_int() {
        assert_eq!(roundtrip_cbor(&Cbor::Integer(42.into())), Value::Int(42));
    }

    #[test]
    fn out_of_range_integer_is_an_error() {
        let too_large = ciborium::value::Integer::try_from(i128::from(i64::MAX) + 1).unwrap();
        assert!(decode_cbor_value(&Cbor::Integer(too_large)).is_err());
    }

    #[test]
    fn null_maps_to_nil() {
        assert_eq!(roundtrip_cbor(&Cbor::Null), Value::Nil);
    }

    #[test]
    fn array_maps_to_proper_list_recursively() {
        let cbor = Cbor::Array(vec![
            Cbor::Text(":enter".to_string()),
            Cbor::Text("ticket-123".to_string()),
        ]);
        let v = roundtrip_cbor(&cbor);
        assert_eq!(
            v,
            Value::list(vec![Value::symbol(":enter"), Value::str("ticket-123")])
        );
    }

    #[test]
    fn map_content_roundtrips() {
        let cbor = Cbor::Map(vec![(Cbor::Text("a".to_string()), Cbor::Integer(1.into()))]);
        let mut expected = BTreeMap::new();
        expected.insert("a".to_string(), Value::Int(1));
        assert_eq!(roundtrip_cbor(&cbor), Value::Map(expected));
    }

    #[test]
    fn map_rejects_non_text_keys() {
        let cbor = Cbor::Map(vec![(Cbor::Integer(1.into()), Cbor::Text("x".to_string()))]);
        assert!(decode_cbor_value(&cbor).is_err());
    }

    #[test]
    fn nested_map_roundtrips() {
        let mut inner = BTreeMap::new();
        inner.insert("north".to_string(), Value::str("did:ma:abc#exit"));
        let mut outer = BTreeMap::new();
        outer.insert("exits".to_string(), Value::Map(inner));
        outer.insert("dark".to_string(), Value::Bool(false));
        let original = Value::Map(outer);
        let bytes = encode(&original).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn encode_rejects_procedures() {
        let env = crate::new_root_env();
        let car = env.lookup("car").unwrap();
        assert!(encode(&car).is_err());
    }

    #[test]
    fn encode_rejects_non_atom_symbols() {
        assert!(encode(&Value::symbol("foo")).is_err());
        assert_eq!(
            decode(&encode(&Value::symbol(":ok")).unwrap()).unwrap(),
            Value::symbol(":ok")
        );
    }

    #[test]
    fn encode_decode_roundtrip_for_state_shaped_values() {
        let original = Value::list(vec![
            Value::str("hello"),
            Value::Int(42),
            Value::Bool(true),
            Value::Nil,
        ]);
        let bytes = encode(&original).unwrap();
        let decoded = decode(&bytes).unwrap();
        assert_eq!(original, decoded);
    }
}
