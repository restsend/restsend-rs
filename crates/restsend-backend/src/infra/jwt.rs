//! Minimal HS256 JWT verification (no external deps).
//!
//! Supports the common server-side flow: an external issuer signs tokens;
//! restsend only verifies them and extracts the user id claim.

const B64URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn b64url_decode(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits = 0u32;
    for ch in input.bytes() {
        if ch == b'=' {
            break;
        }
        let val = B64URL.iter().position(|c| *c == ch)? as u32;
        buffer = (buffer << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xFF) as u8);
        }
    }
    Some(out)
}

fn b64url_encode(input: &[u8]) -> String {
    let mut out = String::new();
    let mut chunks = input.chunks(3);
    for chunk in chunks.by_ref() {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64URL[(n >> 18) as usize & 63] as char);
        out.push(B64URL[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            B64URL[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64URL[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    use sha2::digest::generic_array::GenericArray;
    use sha2::{Digest, Sha256};

    const BLOCK: usize = 64;
    let mut key_block = [0u8; BLOCK];
    if key.len() > BLOCK {
        let hash = Sha256::digest(key);
        key_block[..32].copy_from_slice(&hash);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_hash: GenericArray<u8, sha2::digest::typenum::U32> = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    outer.finalize().into()
}

/// Verify an HS256 JWT and return the payload claims if valid & unexpired.
pub fn verify_hs256(token: &str, secret: &str) -> Option<serde_json::Value> {
    let mut parts = token.split('.');
    let header_b64 = parts.next()?;
    let payload_b64 = parts.next()?;
    let signature_b64 = parts.next()?;
    if parts.next().is_some() {
        return None;
    }

    let signing_input = format!("{header_b64}.{payload_b64}");
    let expected = hmac_sha256(secret.as_bytes(), signing_input.as_bytes());
    let signature = b64url_decode(signature_b64)?;
    if signature.as_slice() != expected {
        return None;
    }

    // header must be HS256
    let header_bytes = b64url_decode(header_b64)?;
    let header: serde_json::Value = serde_json::from_slice(&header_bytes).ok()?;
    if header.get("alg").and_then(|v| v.as_str()) != Some("HS256") {
        return None;
    }

    let payload_bytes = b64url_decode(payload_b64)?;
    let payload: serde_json::Value = serde_json::from_slice(&payload_bytes).ok()?;

    // exp check (unix seconds) when present
    if let Some(exp) = payload.get("exp").and_then(|v| v.as_i64()) {
        let now = chrono::Utc::now().timestamp();
        if now >= exp {
            return None;
        }
    }
    // nbf check when present
    if let Some(nbf) = payload.get("nbf").and_then(|v| v.as_i64()) {
        let now = chrono::Utc::now().timestamp();
        if now < nbf {
            return None;
        }
    }
    Some(payload)
}

#[cfg(test)]
pub(crate) fn sign_token(payload: &serde_json::Value, secret: &str) -> String {
    let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
    let signing_input = format!(
        "{}.{}",
        b64url_encode(header.to_string().as_bytes()),
        b64url_encode(payload.to_string().as_bytes())
    );
    let sig = hmac_sha256(secret.as_bytes(), signing_input.as_bytes());
    format!("{signing_input}.{}", b64url_encode(&sig))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b64url_encode_json(value: &serde_json::Value) -> String {
        b64url_encode(value.to_string().as_bytes())
    }

    fn sign(payload: &serde_json::Value, secret: &str) -> String {
        let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
        let signing_input = format!(
            "{}.{}",
            b64url_encode_json(&header),
            b64url_encode_json(payload)
        );
        let sig = hmac_sha256(secret.as_bytes(), signing_input.as_bytes());
        format!("{signing_input}.{}", b64url_encode(&sig))
    }

    #[test]
    fn base64url_roundtrip() {
        let data = b"hello jwt world";
        assert_eq!(b64url_decode(&b64url_encode(data)).as_deref(), Some(data.as_slice()));
    }

    #[test]
    fn verifies_valid_token_and_extracts_claim() {
        let payload = serde_json::json!({"uid": "alice", "exp": 4102444800i64});
        let token = sign(&payload, "secret");
        let claims = verify_hs256(&token, "secret").expect("valid");
        assert_eq!(claims.get("uid").and_then(|v| v.as_str()), Some("alice"));
    }

    #[test]
    fn rejects_bad_signature() {
        let payload = serde_json::json!({"uid": "alice"});
        let token = sign(&payload, "secret");
        assert!(verify_hs256(&token, "wrong").is_none());
    }

    #[test]
    fn rejects_expired_token() {
        let payload = serde_json::json!({"uid": "alice", "exp": 1000});
        let token = sign(&payload, "secret");
        assert!(verify_hs256(&token, "secret").is_none());
    }

    #[test]
    fn rejects_unknown_alg() {
        let header = b64url_encode(br#"{"alg":"none"}"#);
        let payload = b64url_encode(br#"{"uid":"alice"}"#);
        let token = format!("{header}.{payload}.");
        assert!(verify_hs256(&token, "secret").is_none());
    }
}
