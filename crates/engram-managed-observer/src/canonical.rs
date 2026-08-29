use std::collections::HashSet;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub(crate) const MAX_SAFE_JSON_INTEGER: u64 = 9_007_199_254_740_991;
const MAX_JSON_DEPTH: usize = 32;
const MAX_JSON_NODES: usize = 20_000;

#[derive(Debug, Error)]
pub(crate) enum CanonicalJsonError {
    #[error("strict JSON decoding failed")]
    Decode,
    #[error("JSON value is outside the portable domain")]
    Domain,
    #[error("canonical JSON encoding failed")]
    Encode,
}

struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictValueVisitor)
    }
}

struct StrictValueVisitor;

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = StrictValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("one strict JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value.unsigned_abs() > MAX_SAFE_JSON_INTEGER {
            return Err(E::custom("integer exceeds the portable exact range"));
        }
        Ok(StrictValue(Value::Number(Number::from(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value > MAX_SAFE_JSON_INTEGER {
            return Err(E::custom("integer exceeds the portable exact range"));
        }
        Ok(StrictValue(Value::Number(Number::from(value))))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if !value.is_finite() || value.abs() > 1.0e300 || (value == 0.0 && value.is_sign_negative())
        {
            return Err(E::custom("number is outside the portable finite range"));
        }
        Number::from_f64(value)
            .map(Value::Number)
            .map(StrictValue)
            .ok_or_else(|| E::custom("number is not JSON representable"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        validate_string(value).map_err(E::custom)?;
        Ok(StrictValue(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        validate_string(&value).map_err(E::custom)?;
        Ok(StrictValue(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(StrictValue(value)) = sequence.next_element::<StrictValue>()? {
            values.push(value);
            if values.len() > MAX_JSON_NODES {
                return Err(de::Error::custom("JSON sequence exceeds the node bound"));
            }
        }
        Ok(StrictValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        let mut seen = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            validate_string(&key).map_err(de::Error::custom)?;
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom("JSON object contains a duplicate member"));
            }
            let StrictValue(value) = map.next_value::<StrictValue>()?;
            values.insert(key, value);
            if values.len() > MAX_JSON_NODES {
                return Err(de::Error::custom("JSON object exceeds the node bound"));
            }
        }
        Ok(StrictValue(Value::Object(values)))
    }
}

fn validate_string(value: &str) -> Result<(), &'static str> {
    if value.chars().any(|character| {
        let codepoint = character as u32;
        character == '\u{fffd}'
            || (0xfdd0..=0xfdef).contains(&codepoint)
            || (codepoint & 0xffff == 0xfffe)
            || (codepoint & 0xffff == 0xffff)
            || (codepoint < 0x20 && !matches!(character, '\t' | '\n' | '\r'))
            || (0x7f..=0x9f).contains(&codepoint)
    }) {
        return Err("string contains a nonportable Unicode scalar");
    }
    Ok(())
}

fn validate_structure(root: &Value) -> Result<(), CanonicalJsonError> {
    let mut stack = vec![(root, 1_usize)];
    let mut nodes = 0_usize;
    while let Some((value, depth)) = stack.pop() {
        nodes = nodes.checked_add(1).ok_or(CanonicalJsonError::Domain)?;
        if nodes > MAX_JSON_NODES || depth > MAX_JSON_DEPTH {
            return Err(CanonicalJsonError::Domain);
        }
        match value {
            Value::Null | Value::Bool(_) => {}
            Value::Number(number) => validate_number(number)?,
            Value::String(value) => {
                validate_string(value).map_err(|_| CanonicalJsonError::Domain)?;
            }
            Value::Array(values) => {
                stack.extend(values.iter().map(|value| (value, depth + 1)));
            }
            Value::Object(values) => {
                for (key, value) in values {
                    nodes = nodes.checked_add(1).ok_or(CanonicalJsonError::Domain)?;
                    if nodes > MAX_JSON_NODES || depth + 1 > MAX_JSON_DEPTH {
                        return Err(CanonicalJsonError::Domain);
                    }
                    validate_string(key).map_err(|_| CanonicalJsonError::Domain)?;
                    stack.push((value, depth + 1));
                }
            }
        }
    }
    Ok(())
}

fn validate_number(number: &Number) -> Result<(), CanonicalJsonError> {
    if number.is_i64() {
        return number
            .as_i64()
            .filter(|value| value.unsigned_abs() <= MAX_SAFE_JSON_INTEGER)
            .map(|_| ())
            .ok_or(CanonicalJsonError::Domain);
    }
    if number.is_u64() {
        return number
            .as_u64()
            .filter(|value| *value <= MAX_SAFE_JSON_INTEGER)
            .map(|_| ())
            .ok_or(CanonicalJsonError::Domain);
    }
    let value = number.as_f64().ok_or(CanonicalJsonError::Domain)?;
    if value.is_finite() && value.abs() <= 1.0e300 && !(value == 0.0 && value.is_sign_negative()) {
        Ok(())
    } else {
        Err(CanonicalJsonError::Domain)
    }
}

pub(crate) fn strict_json(payload: &[u8]) -> Result<Value, CanonicalJsonError> {
    let mut deserializer = serde_json::Deserializer::from_slice(payload);
    let StrictValue(value) =
        StrictValue::deserialize(&mut deserializer).map_err(|_| CanonicalJsonError::Decode)?;
    deserializer.end().map_err(|_| CanonicalJsonError::Decode)?;
    validate_structure(&value)?;
    Ok(value)
}

pub(crate) fn to_value<T: Serialize>(value: &T) -> Result<Value, CanonicalJsonError> {
    let value = serde_json::to_value(value).map_err(|_| CanonicalJsonError::Encode)?;
    validate_structure(&value)?;
    Ok(value)
}

pub(crate) fn canonical_json(value: &Value) -> Result<Vec<u8>, CanonicalJsonError> {
    validate_structure(value)?;
    let mut output = Vec::new();
    encode_value(value, &mut output)?;
    Ok(output)
}

fn encode_value(value: &Value, output: &mut Vec<u8>) -> Result<(), CanonicalJsonError> {
    match value {
        Value::Null => output.extend_from_slice(b"null"),
        Value::Bool(false) => output.extend_from_slice(b"false"),
        Value::Bool(true) => output.extend_from_slice(b"true"),
        Value::Number(number) => output.extend_from_slice(number.to_string().as_bytes()),
        Value::String(value) => {
            let encoded = serde_json::to_string(value).map_err(|_| CanonicalJsonError::Encode)?;
            output.extend_from_slice(encoded.as_bytes());
        }
        Value::Array(values) => {
            output.push(b'[');
            for (index, child) in values.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                encode_value(child, output)?;
            }
            output.push(b']');
        }
        Value::Object(values) => {
            output.push(b'{');
            let mut keys: Vec<&String> = values.keys().collect();
            keys.sort_unstable();
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                let encoded_key =
                    serde_json::to_string(key).map_err(|_| CanonicalJsonError::Encode)?;
                output.extend_from_slice(encoded_key.as_bytes());
                output.push(b':');
                let child = values.get(*key).ok_or(CanonicalJsonError::Encode)?;
                encode_value(child, output)?;
            }
            output.push(b'}');
        }
    }
    Ok(())
}

pub(crate) fn sha256_bytes(payload: &[u8]) -> String {
    lower_hex(&Sha256::digest(payload))
}

pub(crate) fn sha256_value(value: &Value) -> Result<String, CanonicalJsonError> {
    canonical_json(value).map(|payload| sha256_bytes(&payload))
}

pub(crate) fn sha256_domain(
    domain: &str,
    payloads: &[&[u8]],
) -> Result<String, CanonicalJsonError> {
    let mut digest = Sha256::new();
    digest.update(domain.as_bytes());
    digest.update([0]);
    for payload in payloads {
        let length = u64::try_from(payload.len()).map_err(|_| CanonicalJsonError::Domain)?;
        digest.update(length.to_be_bytes());
        digest.update(payload);
    }
    Ok(lower_hex(&digest.finalize()))
}

pub(crate) fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct FloatCorpus {
        schema_version: String,
        canonicalizer: String,
        cases: Vec<FloatCase>,
        randomized: RandomizedFloatCorpus,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct RandomizedFloatCorpus {
        algorithm: String,
        seed_hex: String,
        sample_count: usize,
        accepted_count: usize,
        transcript: String,
        transcript_sha256: String,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct FloatCase {
        id: String,
        binary64_be_hex: String,
        portable: bool,
        canonical_json: Option<String>,
    }

    #[test]
    fn strict_json_rejects_duplicate_members() {
        let result = strict_json(br#"{"a":1,"a":2}"#);

        assert!(matches!(result, Err(CanonicalJsonError::Decode)));
    }

    #[test]
    fn canonical_json_orders_object_members() {
        let value = strict_json(br#"{"z":1,"a":[true,null]}"#).expect("valid fixture");

        assert_eq!(
            canonical_json(&value).expect("canonical fixture"),
            br#"{"a":[true,null],"z":1}"#
        );
    }

    #[test]
    fn strict_json_rejects_negative_zero() {
        let result = strict_json(br#"{"value":-0.0}"#);

        assert!(matches!(result, Err(CanonicalJsonError::Decode)));
    }

    #[test]
    fn generated_values_reject_unsafe_numbers_and_nonportable_unicode() {
        for value in [
            Value::Number(Number::from(9_007_199_254_740_992_u64)),
            Value::Number(Number::from(-9_007_199_254_740_992_i64)),
            Value::Number(Number::from_f64(-0.0).expect("negative zero number")),
            Value::Number(
                Number::from_f64(f64::from_bits(0x7e37_e43c_8800_759d)).expect("large number"),
            ),
            Value::String("replacement-\u{fffd}".to_owned()),
            Value::String("noncharacter-\u{fdd0}".to_owned()),
        ] {
            assert!(matches!(
                canonical_json(&value),
                Err(CanonicalJsonError::Domain)
            ));
        }
    }

    #[test]
    fn object_keys_and_values_share_the_host_node_budget() {
        let accepted = Value::Object(
            (0..9_999)
                .map(|index| (format!("key-{index:05}"), Value::Null))
                .collect(),
        );
        canonical_json(&accepted).expect("19,999 nodes remain in bounds");
        let rejected = Value::Object(
            (0..10_000)
                .map(|index| (format!("key-{index:05}"), Value::Null))
                .collect(),
        );
        assert!(matches!(
            canonical_json(&rejected),
            Err(CanonicalJsonError::Domain)
        ));
    }

    #[test]
    fn shared_engram_binary64_corpus_matches_exactly() {
        let corpus: FloatCorpus = serde_json::from_slice(include_bytes!(
            "../../../integrations/engram/managed-observer/contracts/engram.managed-runtime-finite-float.v1.json"
        ))
        .expect("Engram finite-float corpus");
        assert_eq!(
            corpus.schema_version,
            "engram.managed-runtime-finite-float.v1"
        );
        assert_eq!(corpus.canonicalizer, "engram.managed-runtime-json.v1");
        assert_eq!(corpus.cases.len(), 25);
        let mut identifiers = HashSet::new();
        for case in corpus.cases {
            assert!(identifiers.insert(case.id));
            let bits = u64::from_str_radix(&case.binary64_be_hex, 16).expect("binary64 bits");
            let number = Number::from_f64(f64::from_bits(bits)).expect("finite corpus value");
            let result = canonical_json(&Value::Number(number));
            if case.portable {
                assert_eq!(
                    result.expect("portable binary64 value"),
                    case.canonical_json.expect("portable spelling").as_bytes()
                );
            } else {
                assert!(case.canonical_json.is_none());
                assert!(matches!(result, Err(CanonicalJsonError::Domain)));
            }
        }

        let randomized = corpus.randomized;
        assert_eq!(randomized.algorithm, "splitmix64-v1");
        assert_eq!(
            randomized.transcript,
            "lowercase-binary64-hex:canonical-json-or-rejected\\n"
        );
        let mut state = u64::from_str_radix(&randomized.seed_hex, 16).expect("SplitMix64 seed");
        let mut accepted = 0_usize;
        let mut transcript = Vec::new();
        for _ in 0..randomized.sample_count {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut value_bits = state;
            value_bits = (value_bits ^ (value_bits >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            value_bits = (value_bits ^ (value_bits >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            value_bits ^= value_bits >> 31;
            let rendered = Number::from_f64(f64::from_bits(value_bits))
                .ok_or(CanonicalJsonError::Domain)
                .and_then(|number| canonical_json(&Value::Number(number)))
                .map_or_else(
                    |_| "rejected".to_owned(),
                    |bytes| String::from_utf8(bytes).expect("canonical number is ASCII"),
                );
            if rendered != "rejected" {
                accepted += 1;
            }
            transcript.extend_from_slice(format!("{value_bits:016x}:{rendered}\n").as_bytes());
        }
        assert_eq!(accepted, randomized.accepted_count);
        assert_eq!(sha256_bytes(&transcript), randomized.transcript_sha256);
    }
}
