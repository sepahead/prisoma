//! Operator-paste PSK HMAC-SHA256 pairing for the Engram host TCP bridge.
//!
//! The bridge generates one 32-byte startup secret from the operating-system
//! CSPRNG and prints it exactly once to its controlling stderr. Engram never
//! transmits the secret. Both peers prove possession through HMAC-SHA256
//! challenge-response proofs bound to the active profile, the exact JSON-RPC
//! request id text, fresh 32-byte nonces, and (for the server proof) the
//! RFC 8785 JCS hash of the session result without its `pairing` member.
//!
//! Pairing is startup-secret-possession evidence only. It is not same-user,
//! same-terminal, process, build, or commit attestation: a pasted secret can
//! be forwarded or stolen.

use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

/// Public pairing mechanism constant mirrored by the reviewed manifest.
pub const PAIRING_MECHANISM: &str = "operator-paste-psk-hmac-sha256-v1";
/// Public secret-format constant mirrored by the reviewed manifest.
pub const PAIRING_SECRET_FORMAT: &str = "engp1-base64url-256";
/// Public pairing-scope constant mirrored by the reviewed manifest.
pub const PAIRING_SCOPE: &str = "single-successful-tcp-connection";
/// Announcement prefix of the operator-paste secret token.
pub const PAIRING_SECRET_PREFIX: &str = "engp1_";
/// JSON-RPC error code returned for every rejected pairing.
pub const PAIRING_ERROR_CODE: i64 = -32001;
/// JSON-RPC error message returned for every rejected pairing. The wire error
/// never carries a `data` member or a rejection reason.
pub const PAIRING_ERROR_MESSAGE: &str = "pairing rejected";
/// Finite accepted-connection budget declared by the reviewed manifest.
pub const MAX_PAIRING_ATTEMPTS: usize = 8;
/// Declared `bridge.describe` request-payload contract for paired sessions.
pub const PAIRED_BRIDGE_SESSION_REQUEST_PAYLOAD: &str = "engram.bridge-session-request.v2";

const CLIENT_PROOF_LABEL: &[u8] = b"engram-pair-v1\0client\0";
const SERVER_PROOF_LABEL: &[u8] = b"engram-pair-v1\0server\0";
const NONCE_BYTES: usize = 32;
const ENCODED_32_BYTE_LEN: usize = 43;

/// One 32-byte startup pairing secret. `Debug` is redacted and the raw bytes
/// never appear in provenance, receipts, or wire messages.
pub struct PairingSecret([u8; 32]);

impl fmt::Debug for PairingSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PairingSecret(redacted)")
    }
}

impl PairingSecret {
    /// Generate a fresh secret from the operating-system CSPRNG. Callers must
    /// fail before listening when this returns an error.
    pub fn generate() -> Result<Self> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes)
            .map_err(|error| anyhow::anyhow!("operating-system CSPRNG failed: {error}"))?;
        Ok(Self(bytes))
    }

    /// Build a secret from raw bytes (tests and the announcement round trip).
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Parse an operator-paste token matching `^engp1_[A-Za-z0-9_-]{43}$`.
    pub fn from_token(token: &str) -> Result<Self> {
        let Some(encoded) = token.strip_prefix(PAIRING_SECRET_PREFIX) else {
            bail!("pairing token must start with {PAIRING_SECRET_PREFIX}");
        };
        let bytes = decode_32_byte_base64url(encoded)
            .context("pairing token must encode exactly 32 base64url bytes")?;
        Ok(Self(bytes))
    }

    /// Render the operator-paste announcement token `engp1_<43 chars>`.
    pub fn announcement_token(&self) -> String {
        format!("{PAIRING_SECRET_PREFIX}{}", URL_SAFE_NO_PAD.encode(self.0))
    }

    fn key(&self) -> &[u8; 32] {
        &self.0
    }

    /// Raw key bytes for in-crate pairing tests that build client proofs. Not
    /// part of the wire contract; production code never reads the raw secret.
    #[doc(hidden)]
    pub fn key_for_test(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Bridge-lifetime pairing budget and single-successful-connection binding.
#[derive(Debug)]
pub struct BridgePairingGuard {
    secret: PairingSecret,
    max_attempts: usize,
    attempts_used: usize,
    failed_attempts: usize,
    bound: bool,
}

impl BridgePairingGuard {
    pub fn new(secret: PairingSecret, max_attempts: usize) -> Self {
        Self {
            secret,
            max_attempts,
            attempts_used: 0,
            failed_attempts: 0,
            bound: false,
        }
    }

    pub fn secret(&self) -> &PairingSecret {
        &self.secret
    }

    /// Monotonic accepted-connection units consumed so far.
    pub fn attempts_used(&self) -> usize {
        self.attempts_used
    }

    pub fn max_attempts(&self) -> usize {
        self.max_attempts
    }

    /// True once every accepted-connection unit is consumed.
    pub fn attempts_exhausted(&self) -> bool {
        self.attempts_used >= self.max_attempts
    }

    /// True once failed connections alone latch the bridge closed.
    pub fn latched(&self) -> bool {
        self.failed_attempts >= self.max_attempts
    }

    /// True after the first valid proof bound the secret to one connection.
    pub fn bound(&self) -> bool {
        self.bound
    }

    /// Consume one pairing unit for a newly accepted connection. Timeouts,
    /// wrong proofs, and post-binding rejections all consume a unit.
    pub fn begin_attempt(&mut self) -> Result<()> {
        if self.attempts_exhausted() {
            bail!(
                "pairing attempt budget exhausted: {} of {} units consumed",
                self.attempts_used,
                self.max_attempts
            );
        }
        self.attempts_used += 1;
        Ok(())
    }

    /// Atomically bind the secret to the current connection.
    pub fn mark_bound(&mut self) {
        self.bound = true;
    }

    /// Record one connection that ended without a successful pairing.
    pub fn record_failed_attempt(&mut self) {
        self.failed_attempts = self.failed_attempts.saturating_add(1);
    }
}

/// A validated `bridge.session` pairing request.
#[derive(Debug, Clone)]
pub struct VerifiedPairing {
    pub request_id_json: String,
    pub client_nonce: [u8; 32],
}

/// Validate and verify the `pairing` parameters of one parsed JSON-RPC
/// `bridge.session` request value. `used_client_nonces` holds every client
/// nonce already accepted on the same TCP connection.
pub fn verify_bridge_session_pairing(
    secret: &PairingSecret,
    profile: &str,
    request: &Value,
    used_client_nonces: &BTreeSet<[u8; 32]>,
) -> Result<VerifiedPairing> {
    let Value::Object(request) = request else {
        bail!("pairing request must be one JSON object");
    };
    if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        bail!("pairing request jsonrpc must be exactly \"2.0\"");
    }
    let id = match request.get("id") {
        Some(id @ (Value::String(_) | Value::Number(_))) => id,
        _ => bail!("pairing request id must be a JSON string or number"),
    };
    if request.get("method").and_then(Value::as_str) != Some("bridge.session") {
        bail!("the first request on a paired connection must be bridge.session");
    }
    let Some(Value::Object(params)) = request.get("params") else {
        bail!("bridge.session pairing params must be an object");
    };
    if params.len() != 1 || !params.contains_key("pairing") {
        bail!("bridge.session params must contain exactly the pairing member");
    }
    let Some(Value::Object(pairing)) = params.get("pairing") else {
        bail!("bridge.session pairing member must be an object");
    };
    let expected_keys = ["client_nonce", "client_proof", "mechanism"];
    if pairing.len() != expected_keys.len()
        || !expected_keys.iter().all(|key| pairing.contains_key(*key))
    {
        bail!("pairing must contain exactly mechanism, client_nonce, and client_proof");
    }
    if pairing.get("mechanism").and_then(Value::as_str) != Some(PAIRING_MECHANISM) {
        bail!("pairing mechanism must be {PAIRING_MECHANISM}");
    }
    let client_nonce = pairing
        .get("client_nonce")
        .and_then(Value::as_str)
        .context("pairing client_nonce must be a string")
        .and_then(decode_32_byte_base64url)
        .context("pairing client_nonce must encode exactly 32 base64url bytes")?;
    if used_client_nonces.contains(&client_nonce) {
        bail!("pairing client_nonce must never repeat within a connection");
    }
    let client_proof = pairing
        .get("client_proof")
        .and_then(Value::as_str)
        .context("pairing client_proof must be a string")
        .and_then(decode_32_byte_base64url)
        .context("pairing client_proof must encode exactly 32 base64url bytes")?;
    let request_id_json =
        serde_json::to_string(id).context("failed to serialize the JSON-RPC id")?;
    let expected = hmac_sha256(
        secret.key(),
        &client_proof_message(profile, &request_id_json, &client_nonce),
    );
    if !constant_time_eq_32(&expected, &client_proof) {
        bail!("pairing client proof mismatch");
    }
    Ok(VerifiedPairing {
        request_id_json,
        client_nonce,
    })
}

/// Build the successful `bridge.session` result `pairing` member. The server
/// proof binds the profile, the exact request id text, both nonces, and the
/// SHA-256 of the RFC 8785 JCS form of `session_core` (the result without its
/// `pairing` member).
pub fn build_server_pairing_member(
    secret: &PairingSecret,
    profile: &str,
    verified: &VerifiedPairing,
    session_core: &Value,
) -> Result<Value> {
    let mut server_nonce = [0u8; NONCE_BYTES];
    getrandom::fill(&mut server_nonce)
        .map_err(|error| anyhow::anyhow!("operating-system CSPRNG failed: {error}"))?;
    let session_core_hash = sha256(jcs_canonicalize(session_core)?.as_bytes());
    let server_proof = hmac_sha256(
        secret.key(),
        &server_proof_message(
            profile,
            &verified.request_id_json,
            &verified.client_nonce,
            &server_nonce,
            &session_core_hash,
        ),
    );
    Ok(json!({
        "mechanism": PAIRING_MECHANISM,
        "client_nonce": URL_SAFE_NO_PAD.encode(verified.client_nonce),
        "server_nonce": URL_SAFE_NO_PAD.encode(server_nonce),
        "server_proof": URL_SAFE_NO_PAD.encode(server_proof),
    }))
}

/// Exact HMAC input for the client proof.
pub fn client_proof_message(
    profile: &str,
    request_id_json: &str,
    client_nonce: &[u8; 32],
) -> Vec<u8> {
    let mut message = Vec::with_capacity(
        CLIENT_PROOF_LABEL.len() + profile.len() + request_id_json.len() + NONCE_BYTES + 2,
    );
    message.extend_from_slice(CLIENT_PROOF_LABEL);
    message.extend_from_slice(profile.as_bytes());
    message.push(0);
    message.extend_from_slice(request_id_json.as_bytes());
    message.push(0);
    message.extend_from_slice(client_nonce);
    message
}

/// Exact HMAC input for the server proof.
pub fn server_proof_message(
    profile: &str,
    request_id_json: &str,
    client_nonce: &[u8; 32],
    server_nonce: &[u8; 32],
    session_core_sha256: &[u8; 32],
) -> Vec<u8> {
    let mut message = Vec::with_capacity(
        SERVER_PROOF_LABEL.len() + profile.len() + request_id_json.len() + 3 * NONCE_BYTES + 2,
    );
    message.extend_from_slice(SERVER_PROOF_LABEL);
    message.extend_from_slice(profile.as_bytes());
    message.push(0);
    message.extend_from_slice(request_id_json.as_bytes());
    message.push(0);
    message.extend_from_slice(client_nonce);
    message.extend_from_slice(server_nonce);
    message.extend_from_slice(session_core_sha256);
    message
}

/// Decode exactly 43 unpadded base64url characters to 32 bytes.
pub fn decode_32_byte_base64url(encoded: &str) -> Result<[u8; 32]> {
    if encoded.len() != ENCODED_32_BYTE_LEN
        || !encoded
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        bail!("value must be exactly {ENCODED_32_BYTE_LEN} unpadded base64url characters");
    }
    let decoded = URL_SAFE_NO_PAD
        .decode(encoded)
        .context("invalid base64url value")?;
    let bytes: [u8; 32] = decoded
        .try_into()
        .map_err(|_| anyhow::anyhow!("value must decode to exactly 32 bytes"))?;
    Ok(bytes)
}

/// SHA-256 of one byte string.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// HMAC-SHA256 (RFC 2104) over the 64-byte SHA-256 block size.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut key_block = [0u8; BLOCK];
    if key.len() > BLOCK {
        key_block[..32].copy_from_slice(&sha256(key));
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha256::new();
    let mut inner_pad = [0u8; BLOCK];
    let mut outer_pad = [0u8; BLOCK];
    for index in 0..BLOCK {
        inner_pad[index] = key_block[index] ^ 0x36;
        outer_pad[index] = key_block[index] ^ 0x5c;
    }
    inner.update(inner_pad);
    inner.update(message);
    let inner_hash: [u8; 32] = inner.finalize().into();
    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner_hash);
    outer.finalize().into()
}

/// Constant-time 32-byte comparison.
pub fn constant_time_eq_32(left: &[u8; 32], right: &[u8; 32]) -> bool {
    let mut difference = 0u8;
    for index in 0..32 {
        difference |= left[index] ^ right[index];
    }
    std::hint::black_box(difference) == 0
}

/// RFC 8785 JSON Canonicalization Scheme for the bridge's session payloads.
///
/// The bridge session result contains only null, booleans, strings, arrays,
/// objects, and integers below 2^53, for which this canonical form is exact.
/// Non-integer or unsafely large numbers fail closed instead of risking a
/// cross-implementation mismatch.
pub fn jcs_canonicalize(value: &Value) -> Result<String> {
    let mut output = String::new();
    write_jcs(value, &mut output)?;
    Ok(output)
}

fn write_jcs(value: &Value, output: &mut String) -> Result<()> {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(true) => output.push_str("true"),
        Value::Bool(false) => output.push_str("false"),
        Value::Number(number) => {
            const MAX_SAFE: u64 = (1 << 53) - 1;
            let rendered = if let Some(unsigned) = number.as_u64() {
                if unsigned > MAX_SAFE {
                    bail!("JCS integer {unsigned} exceeds 2^53-1");
                }
                unsigned.to_string()
            } else if let Some(signed) = number.as_i64() {
                if signed < -((MAX_SAFE) as i64) {
                    bail!("JCS integer {signed} is below -(2^53-1)");
                }
                signed.to_string()
            } else {
                bail!("JCS canonicalization supports integers only, got {number}");
            };
            output.push_str(&rendered);
        }
        Value::String(text) => {
            output.push_str(
                &serde_json::to_string(text).context("failed to serialize a JSON string")?,
            );
        }
        Value::Array(items) => {
            output.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_jcs(item, output)?;
            }
            output.push(']');
        }
        Value::Object(members) => {
            let mut keys: Vec<&String> = members.keys().collect();
            // RFC 8785 sorts members by UTF-16 code units of the raw key.
            keys.sort_by(|left, right| left.encode_utf16().cmp(right.encode_utf16()));
            output.push('{');
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(
                    &serde_json::to_string(key).context("failed to serialize a JSON key")?,
                );
                output.push(':');
                write_jcs(&members[key.as_str()], output)?;
            }
            output.push('}');
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn hmac_sha256_matches_rfc_4231_case_1_and_2() {
        // RFC 4231 test case 1.
        let key = [0x0b_u8; 20];
        let mac = hmac_sha256(&key, b"Hi There");
        assert_eq!(
            hex(&mac),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
        // RFC 4231 test case 2 ("Jefe").
        let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hex(&mac),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn hmac_sha256_hashes_keys_longer_than_one_block() {
        // RFC 4231 test case 6 uses a 131-byte key.
        let key = [0xaa_u8; 131];
        let mac = hmac_sha256(
            &key,
            b"Test Using Larger Than Block-Size Key - Hash Key First",
        );
        assert_eq!(
            hex(&mac),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn jcs_canonicalizes_sorted_ascii_members_and_escapes() {
        let value = serde_json::json!({
            "b": [1, 2, false],
            "a": "line\nbreak\u{1f}",
            "nested": {"z": null, "y": "√"},
        });
        assert_eq!(
            jcs_canonicalize(&value).unwrap(),
            "{\"a\":\"line\\nbreak\\u001f\",\"b\":[1,2,false],\"nested\":{\"y\":\"√\",\"z\":null}}"
        );
    }

    #[test]
    fn jcs_rejects_non_integer_numbers() {
        let value = serde_json::json!({"pi": 3.5});
        assert!(jcs_canonicalize(&value).is_err());
        let value = serde_json::json!(9007199254740992_u64);
        assert!(jcs_canonicalize(&value).is_err());
    }

    #[test]
    fn secret_token_round_trips_and_enforces_grammar() {
        let secret = PairingSecret::from_bytes([7u8; 32]);
        let token = secret.announcement_token();
        assert!(token.starts_with("engp1_"));
        assert_eq!(token.len(), "engp1_".len() + 43);
        assert!(token["engp1_".len()..]
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'));
        let parsed = PairingSecret::from_token(&token).unwrap();
        assert_eq!(parsed.key(), secret.key());

        assert!(PairingSecret::from_token("engp1_short").is_err());
        assert!(PairingSecret::from_token(&token.replace("engp1_", "engp2_")).is_err());
        let padded = format!("{}=", &token[..token.len() - 1]);
        assert!(PairingSecret::from_token(&padded).is_err());
    }

    fn paired_request(secret: &PairingSecret, profile: &str, id: &Value, nonce: [u8; 32]) -> Value {
        let request_id_json = serde_json::to_string(id).unwrap();
        let proof = hmac_sha256(
            &secret.0,
            &client_proof_message(profile, &request_id_json, &nonce),
        );
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "bridge.session",
            "params": {
                "pairing": {
                    "mechanism": PAIRING_MECHANISM,
                    "client_nonce": URL_SAFE_NO_PAD.encode(nonce),
                    "client_proof": URL_SAFE_NO_PAD.encode(proof),
                }
            }
        })
    }

    #[test]
    fn verify_accepts_a_valid_client_proof_and_binds_the_exact_id_text() {
        let secret = PairingSecret::from_bytes([9u8; 32]);
        let profile = "engram-host-read-only-v2";
        let request = paired_request(&secret, profile, &json!("engram-extension-1"), [1u8; 32]);

        let verified =
            verify_bridge_session_pairing(&secret, profile, &request, &BTreeSet::new()).unwrap();

        assert_eq!(verified.request_id_json, "\"engram-extension-1\"");
        assert_eq!(verified.client_nonce, [1u8; 32]);

        // Numeric ids serialize as bare JSON numbers.
        let request = paired_request(&secret, profile, &json!(7), [2u8; 32]);
        let verified =
            verify_bridge_session_pairing(&secret, profile, &request, &BTreeSet::new()).unwrap();
        assert_eq!(verified.request_id_json, "7");
    }

    #[test]
    fn verify_rejects_wrong_secret_wrong_profile_and_id_substitution() {
        let secret = PairingSecret::from_bytes([9u8; 32]);
        let profile = "engram-host-read-only-v2";
        let request = paired_request(&secret, profile, &json!("id-1"), [1u8; 32]);

        let mut wrong = [9u8; 32];
        wrong[0] ^= 0x01;
        let wrong_secret = PairingSecret::from_bytes(wrong);
        assert!(
            verify_bridge_session_pairing(&wrong_secret, profile, &request, &BTreeSet::new())
                .is_err()
        );
        assert!(verify_bridge_session_pairing(
            &secret,
            "engram-host-read-only-v1",
            &request,
            &BTreeSet::new()
        )
        .is_err());

        let mut substituted = request.clone();
        substituted["id"] = json!("id-2");
        assert!(
            verify_bridge_session_pairing(&secret, profile, &substituted, &BTreeSet::new())
                .is_err()
        );
    }

    #[test]
    fn verify_rejects_malformed_pairing_shapes() {
        let secret = PairingSecret::from_bytes([9u8; 32]);
        let profile = "engram-host-read-only-v2";
        let valid = paired_request(&secret, profile, &json!("id-1"), [1u8; 32]);

        let mut missing_id = valid.clone();
        missing_id.as_object_mut().unwrap().remove("id");
        let mut wrong_method = valid.clone();
        wrong_method["method"] = json!("sim.status");
        let mut extra_param = valid.clone();
        extra_param["params"]["other"] = json!(1);
        let mut extra_pairing_key = valid.clone();
        extra_pairing_key["params"]["pairing"]["extra"] = json!(1);
        let mut wrong_mechanism = valid.clone();
        wrong_mechanism["params"]["pairing"]["mechanism"] = json!("operator-paste-token-v1");
        let mut short_nonce = valid.clone();
        short_nonce["params"]["pairing"]["client_nonce"] = json!("abc");
        for hostile in [
            missing_id,
            wrong_method,
            extra_param,
            extra_pairing_key,
            wrong_mechanism,
            short_nonce,
        ] {
            assert!(
                verify_bridge_session_pairing(&secret, profile, &hostile, &BTreeSet::new())
                    .is_err(),
                "must reject {hostile}"
            );
        }
    }

    #[test]
    fn verify_rejects_a_repeated_client_nonce_within_the_connection() {
        let secret = PairingSecret::from_bytes([9u8; 32]);
        let profile = "engram-host-read-only-v2";
        let request = paired_request(&secret, profile, &json!("id-1"), [4u8; 32]);
        let mut used = BTreeSet::new();
        used.insert([4u8; 32]);

        assert!(verify_bridge_session_pairing(&secret, profile, &request, &used).is_err());
    }

    #[test]
    fn server_pairing_member_verifies_against_the_session_core_hash() {
        let secret = PairingSecret::from_bytes([3u8; 32]);
        let profile = "engram-host-read-only-v2";
        let verified = VerifiedPairing {
            request_id_json: "\"id-1\"".to_string(),
            client_nonce: [5u8; 32],
        };
        let session_core = json!({"run_id": "run", "safe_mode": true, "requests": 2});

        let member = build_server_pairing_member(&secret, profile, &verified, &session_core)
            .expect("server pairing member should build");

        assert_eq!(member["mechanism"], PAIRING_MECHANISM);
        assert_eq!(member["client_nonce"], URL_SAFE_NO_PAD.encode([5u8; 32]));
        let server_nonce =
            decode_32_byte_base64url(member["server_nonce"].as_str().unwrap()).unwrap();
        let server_proof =
            decode_32_byte_base64url(member["server_proof"].as_str().unwrap()).unwrap();
        let core_hash = sha256(jcs_canonicalize(&session_core).unwrap().as_bytes());
        let expected = hmac_sha256(
            &[3u8; 32],
            &server_proof_message(profile, "\"id-1\"", &[5u8; 32], &server_nonce, &core_hash),
        );
        assert!(constant_time_eq_32(&expected, &server_proof));
    }

    #[test]
    fn bridge_pairing_guard_tracks_budget_binding_and_latch() {
        let mut guard = BridgePairingGuard::new(PairingSecret::from_bytes([1u8; 32]), 3);
        assert!(!guard.attempts_exhausted());
        guard.begin_attempt().unwrap();
        guard.record_failed_attempt();
        guard.begin_attempt().unwrap();
        guard.mark_bound();
        assert!(guard.bound());
        guard.begin_attempt().unwrap();
        guard.record_failed_attempt();
        assert!(guard.attempts_exhausted());
        assert!(guard.begin_attempt().is_err());
        assert!(!guard.latched());
        guard.record_failed_attempt();
        assert!(guard.latched());
    }
}
