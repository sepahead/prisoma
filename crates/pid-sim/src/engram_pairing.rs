//! Operator-paste PSK HMAC-SHA256 pairing for the Engram host TCP bridge.
//!
//! The bridge generates one 32-byte startup secret from the operating-system
//! CSPRNG and prints it exactly once to its controlling stderr. Engram never
//! transmits the secret. Both peers prove possession through HMAC-SHA256
//! challenge-response. The proofs bind the active profile, the request ID's JCS
//! form, and fresh 32-byte nonces. The server proof binds the RFC
//! 8785 JCS hash of the session result without its `pairing` member.
//!
//! Pairing is startup-secret-possession evidence only. It is not same-user,
//! same-terminal, process, build, or commit attestation: a pasted secret can
//! be forwarded or stolen.

use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, KeyInit as _, Mac};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;
use zeroize::{Zeroize as _, Zeroizing};

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

impl Drop for PairingSecret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for PairingSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PairingSecret(redacted)")
    }
}

impl PairingSecret {
    /// Generate a fresh secret from the operating-system CSPRNG. Callers must
    /// fail before listening when this returns an error.
    pub fn generate() -> Result<Self> {
        let mut bytes = Zeroizing::new([0u8; 32]);
        getrandom::fill(&mut *bytes)
            .map_err(|error| anyhow::anyhow!("operating-system CSPRNG failed: {error}"))?;
        Ok(Self(*bytes))
    }

    /// Build a secret from caller-supplied bytes.
    ///
    /// Production callers must supply 32 uniformly random bytes from a CSPRNG.
    /// Prefer [`Self::generate`] when the secret originates in this process.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Parse an operator-paste token matching `^engp1_[A-Za-z0-9_-]{43}$`.
    pub fn from_token(token: &str) -> Result<Self> {
        let Some(encoded) = token.strip_prefix(PAIRING_SECRET_PREFIX) else {
            bail!("pairing token must start with {PAIRING_SECRET_PREFIX}");
        };
        let bytes = decode_32_byte_base64url_zeroizing(encoded)
            .context("pairing token must encode exactly 32 base64url bytes")?;
        Ok(Self(*bytes))
    }

    /// Render the operator-paste announcement token `engp1_<43 chars>`.
    pub fn announcement_token(&self) -> Zeroizing<String> {
        let mut token = Zeroizing::new(String::with_capacity(
            PAIRING_SECRET_PREFIX.len() + ENCODED_32_BYTE_LEN,
        ));
        token.push_str(PAIRING_SECRET_PREFIX);
        URL_SAFE_NO_PAD.encode_string(self.0, &mut token);
        token
    }

    fn key(&self) -> &[u8; 32] {
        &self.0
    }

    /// Build the client proof for one canonical JSON-RPC request-id value.
    ///
    /// Pairing accepts only string or safe-integer ids. The same JCS form is
    /// therefore reproducible across conforming JSON implementations.
    pub fn client_proof(
        &self,
        profile: &str,
        request_id: &Value,
        client_nonce: &[u8; 32],
    ) -> Result<[u8; 32]> {
        let request_id_jcs = canonical_request_id(request_id)?;
        Ok(hmac_sha256(
            self.key(),
            &client_proof_message(profile, &request_id_jcs, client_nonce),
        ))
    }

    /// Verify the server proof for a successful paired-session result.
    ///
    /// `session_core` is the result object with its `pairing` member removed.
    pub fn verify_server_proof(
        &self,
        profile: &str,
        request_id: &Value,
        client_nonce: &[u8; 32],
        server_nonce: &[u8; 32],
        session_core: &Value,
        proof: &[u8; 32],
    ) -> Result<()> {
        let request_id_jcs = canonical_request_id(request_id)?;
        let session_core_hash = sha256(jcs_canonicalize(session_core)?.as_bytes());
        verify_hmac_sha256(
            self.key(),
            &server_proof_message(
                profile,
                &request_id_jcs,
                client_nonce,
                server_nonce,
                &session_core_hash,
            ),
            proof,
        )
        .context("pairing server proof mismatch")
    }
}

/// Bridge-lifetime pairing budget and single-successful-connection binding.
#[derive(Debug)]
pub(crate) struct BridgePairingGuard {
    secret: PairingSecret,
    max_attempts: usize,
    attempts_used: usize,
    failed_attempts: usize,
    bound: bool,
}

impl BridgePairingGuard {
    pub(crate) fn new(secret: PairingSecret, max_attempts: usize) -> Self {
        Self {
            secret,
            max_attempts,
            attempts_used: 0,
            failed_attempts: 0,
            bound: false,
        }
    }

    pub(crate) fn secret(&self) -> &PairingSecret {
        &self.secret
    }

    /// Monotonic accepted-connection units consumed so far.
    pub(crate) fn attempts_used(&self) -> usize {
        self.attempts_used
    }

    /// True once every accepted-connection unit is consumed.
    pub(crate) fn attempts_exhausted(&self) -> bool {
        self.attempts_used >= self.max_attempts
    }

    /// True once failed connections alone latch the bridge closed.
    pub(crate) fn latched(&self) -> bool {
        self.failed_attempts >= self.max_attempts
    }

    /// True after the first valid proof bound the secret to one connection.
    pub(crate) fn bound(&self) -> bool {
        self.bound
    }

    /// Consume one pairing unit for a newly accepted connection. Timeouts,
    /// wrong proofs, and post-binding rejections all consume a unit.
    pub(crate) fn begin_attempt(&mut self) -> Result<()> {
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
    pub(crate) fn mark_bound(&mut self) {
        self.bound = true;
    }

    /// Record one connection that ended without a successful pairing.
    pub(crate) fn record_failed_attempt(&mut self) {
        self.failed_attempts = self.failed_attempts.saturating_add(1);
    }
}

/// A validated `bridge.session` pairing request.
#[derive(Debug, Clone)]
pub(crate) struct VerifiedPairing {
    /// RFC 8785 JCS form of the validated string or safe-integer request id.
    pub(crate) request_id_jcs: String,
    pub(crate) client_nonce: [u8; 32],
}

/// Validate and verify the `pairing` parameters of one parsed JSON-RPC
/// `bridge.session` request value. `used_client_nonces` holds every client
/// nonce already accepted on the same TCP connection.
pub(crate) fn verify_bridge_session_pairing(
    secret: &PairingSecret,
    profile: &str,
    request: &Value,
    used_client_nonces: &BTreeSet<[u8; 32]>,
) -> Result<VerifiedPairing> {
    let Value::Object(request) = request else {
        bail!("pairing request must be one JSON object");
    };
    let expected_request_keys = ["id", "jsonrpc", "method", "params"];
    if request.len() != expected_request_keys.len()
        || !expected_request_keys
            .iter()
            .all(|key| request.contains_key(*key))
    {
        bail!("pairing request must contain exactly jsonrpc, id, method, and params");
    }
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
    let request_id_jcs = canonical_request_id(id)?;
    verify_hmac_sha256(
        secret.key(),
        &client_proof_message(profile, &request_id_jcs, &client_nonce),
        &client_proof,
    )
    .context("pairing client proof mismatch")?;
    Ok(VerifiedPairing {
        request_id_jcs,
        client_nonce,
    })
}

/// Build the successful `bridge.session` result `pairing` member. The server
/// proof binds the profile, canonical request id, both nonces, and the
/// SHA-256 of the RFC 8785 JCS form of `session_core` (the result without its
/// `pairing` member).
pub(crate) fn build_server_pairing_member(
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
            &verified.request_id_jcs,
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
fn client_proof_message(profile: &str, request_id_jcs: &str, client_nonce: &[u8; 32]) -> Vec<u8> {
    let mut message = Vec::with_capacity(
        CLIENT_PROOF_LABEL.len() + profile.len() + request_id_jcs.len() + NONCE_BYTES + 2,
    );
    message.extend_from_slice(CLIENT_PROOF_LABEL);
    message.extend_from_slice(profile.as_bytes());
    message.push(0);
    message.extend_from_slice(request_id_jcs.as_bytes());
    message.push(0);
    message.extend_from_slice(client_nonce);
    message
}

/// Exact HMAC input for the server proof.
fn server_proof_message(
    profile: &str,
    request_id_jcs: &str,
    client_nonce: &[u8; 32],
    server_nonce: &[u8; 32],
    session_core_sha256: &[u8; 32],
) -> Vec<u8> {
    let mut message = Vec::with_capacity(
        SERVER_PROOF_LABEL.len() + profile.len() + request_id_jcs.len() + 3 * NONCE_BYTES + 2,
    );
    message.extend_from_slice(SERVER_PROOF_LABEL);
    message.extend_from_slice(profile.as_bytes());
    message.push(0);
    message.extend_from_slice(request_id_jcs.as_bytes());
    message.push(0);
    message.extend_from_slice(client_nonce);
    message.extend_from_slice(server_nonce);
    message.extend_from_slice(session_core_sha256);
    message
}

/// Canonical cross-language representation of a pairing request id.
///
/// JSON-RPC permits strings and numbers. The paired profile narrows numbers to
/// safe integers so every conforming JCS implementation derives identical bytes.
fn canonical_request_id(request_id: &Value) -> Result<String> {
    const MAX_SAFE_INTEGER: u64 = (1_u64 << 53) - 1;

    match request_id {
        Value::String(_) => jcs_canonicalize(request_id),
        Value::Number(number)
            if number
                .as_i64()
                .is_some_and(|value| value.unsigned_abs() <= MAX_SAFE_INTEGER)
                || number
                    .as_u64()
                    .is_some_and(|value| value <= MAX_SAFE_INTEGER) =>
        {
            jcs_canonicalize(request_id)
        }
        _ => bail!("pairing request id must be a JSON string or safe integer"),
    }
}

/// Decode exactly 43 unpadded base64url characters to 32 bytes.
fn decode_32_byte_base64url_zeroizing(encoded: &str) -> Result<Zeroizing<[u8; 32]>> {
    if encoded.len() != ENCODED_32_BYTE_LEN
        || !encoded
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        bail!("value must be exactly {ENCODED_32_BYTE_LEN} unpadded base64url characters");
    }
    // A decoder error can occur after it writes part of the output buffer.
    // Keep that buffer guarded on every return path, not only the length check.
    let mut bytes = Zeroizing::new([0u8; 32]);
    let decoded = URL_SAFE_NO_PAD
        .decode_slice(encoded, &mut *bytes)
        .context("invalid base64url value")?;
    if decoded != bytes.len() {
        bail!("value must decode to exactly 32 bytes");
    }
    Ok(bytes)
}

pub(crate) fn decode_32_byte_base64url(encoded: &str) -> Result<[u8; 32]> {
    decode_32_byte_base64url_zeroizing(encoded).map(|bytes| *bytes)
}

/// SHA-256 of one byte string.
fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// HMAC-SHA256 using the RustCrypto implementation.
fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts keys of every length");
    mac.update(message);
    mac.finalize().into_bytes().into()
}

fn verify_hmac_sha256(key: &[u8], message: &[u8], proof: &[u8; 32]) -> Result<()> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts keys of every length");
    mac.update(message);
    mac.verify_slice(proof)
        .map_err(|_| anyhow::anyhow!("HMAC verification failed"))
}

/// RFC 8785 JSON Canonicalization Scheme for the bridge's session payloads.
///
/// The bridge session result contains only null, booleans, strings, arrays,
/// objects, and integers below 2^53, for which this canonical form is exact.
/// Non-integer or unsafely large numbers fail closed instead of risking a
/// cross-implementation mismatch.
fn jcs_canonicalize(value: &Value) -> Result<String> {
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
    fn proof_message_framing_is_exact_and_cross_implementation_stable() {
        let client_nonce = [0x11; 32];
        let server_nonce = [0x22; 32];
        let session_hash = [0x33; 32];

        let mut expected_client = b"engram-pair-v1\0client\0profile\0\"id\"\0".to_vec();
        expected_client.extend_from_slice(&client_nonce);
        assert_eq!(
            client_proof_message("profile", "\"id\"", &client_nonce),
            expected_client
        );

        let mut expected_server = b"engram-pair-v1\0server\0profile\0\"id\"\0".to_vec();
        expected_server.extend_from_slice(&client_nonce);
        expected_server.extend_from_slice(&server_nonce);
        expected_server.extend_from_slice(&session_hash);
        assert_eq!(
            server_proof_message(
                "profile",
                "\"id\"",
                &client_nonce,
                &server_nonce,
                &session_hash,
            ),
            expected_server
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
        let proof = secret.client_proof(profile, id, &nonce).unwrap();
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
    fn verify_accepts_a_valid_client_proof_and_binds_the_canonical_id() {
        let secret = PairingSecret::from_bytes([9u8; 32]);
        let profile = "engram-host-read-only-v2";
        let request = paired_request(&secret, profile, &json!("engram-extension-1"), [1u8; 32]);

        let verified =
            verify_bridge_session_pairing(&secret, profile, &request, &BTreeSet::new()).unwrap();

        assert_eq!(verified.request_id_jcs, "\"engram-extension-1\"");
        assert_eq!(verified.client_nonce, [1u8; 32]);

        // Safe-integer ids have one cross-language JCS representation.
        let request = paired_request(&secret, profile, &json!(7), [2u8; 32]);
        let verified =
            verify_bridge_session_pairing(&secret, profile, &request, &BTreeSet::new()).unwrap();
        assert_eq!(verified.request_id_jcs, "7");
    }

    #[test]
    fn canonical_pairing_id_rejects_floats_and_unsafe_integers() {
        assert_eq!(canonical_request_id(&json!("id-1")).unwrap(), "\"id-1\"");
        assert_eq!(canonical_request_id(&json!(7)).unwrap(), "7");
        assert_eq!(
            canonical_request_id(&json!(9_007_199_254_740_991_u64)).unwrap(),
            "9007199254740991"
        );
        assert_eq!(
            canonical_request_id(&json!(-9_007_199_254_740_991_i64)).unwrap(),
            "-9007199254740991"
        );
        assert!(canonical_request_id(&json!(1.5)).is_err());
        assert!(canonical_request_id(&json!(9_007_199_254_740_992_u64)).is_err());
        assert!(canonical_request_id(&json!(-9_007_199_254_740_992_i64)).is_err());
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
        let mut extra_top_level = valid.clone();
        extra_top_level["extension"] = json!(true);
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
            extra_top_level,
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
            request_id_jcs: "\"id-1\"".to_string(),
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
        assert_eq!(expected, server_proof);
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
