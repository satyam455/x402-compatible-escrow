//! Off-chain signed authorization verdict prototype.
//! The canonical bytes are domain-separated and must not be treated as an onchain
//! service-delivery attestation. Escrow release remains buyer/arbiter-authorized.
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Verdict {
    pub version: u8,
    pub verdict_id: String,
    pub agent_id: String,
    pub buyer_wallet: String,
    pub recipient: String,
    pub amount_base_units: u64,
    pub mint: String,
    pub network: String,
    pub resource_id: String,
    pub nonce: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub decision: String,
    pub key_id: String,
}
#[derive(Debug, Error, PartialEq, Eq)]
pub enum VerifyError {
    #[error("invalid signature")]
    InvalidSignature,
    #[error("verdict expired or not yet valid")]
    Expired,
    #[error("decision is not allow")]
    NotAllowed,
    #[error("nonce already used")]
    NonceUsed,
    #[error("domain mismatch")]
    DomainMismatch,
}
pub fn canonical_bytes(v: &Verdict) -> Vec<u8> {
    let payload = serde_json::to_vec(v).expect("Verdict is serializable");
    let mut out = b"AGENTPAY_VERDICT_V1\0".to_vec();
    out.extend(Sha256::digest(payload));
    out
}
pub fn verify(
    v: &Verdict,
    signature: &[u8],
    public_key: &[u8; 32],
    now: i64,
    expected_domain: &str,
    used_nonce: bool,
) -> Result<(), VerifyError> {
    if expected_domain != "agentpay/solana-devnet" {
        return Err(VerifyError::DomainMismatch);
    }
    if v.decision != "ALLOW" {
        return Err(VerifyError::NotAllowed);
    }
    if now < v.issued_at || now >= v.expires_at {
        return Err(VerifyError::Expired);
    }
    if used_nonce {
        return Err(VerifyError::NonceUsed);
    }
    let key = VerifyingKey::from_bytes(public_key).map_err(|_| VerifyError::InvalidSignature)?;
    let sig = Signature::from_slice(signature).map_err(|_| VerifyError::InvalidSignature)?;
    key.verify(&canonical_bytes(v), &sig)
        .map_err(|_| VerifyError::InvalidSignature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    fn sample() -> Verdict {
        Verdict {
            version: 1,
            verdict_id: "v-1".into(),
            agent_id: "research".into(),
            buyer_wallet: "buyer".into(),
            recipient: "seller".into(),
            amount_base_units: 10_000,
            mint: "usdc".into(),
            network: "solana:devnet".into(),
            resource_id: "report".into(),
            nonce: "n-1".into(),
            issued_at: 100,
            expires_at: 200,
            decision: "ALLOW".into(),
            key_id: "k-1".into(),
        }
    }
    #[test]
    fn accepts_valid_signature() {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let v = sample();
        let sig = sk.sign(&canonical_bytes(&v));
        assert!(verify(
            &v,
            &sig.to_bytes(),
            sk.verifying_key().as_bytes(),
            150,
            "agentpay/solana-devnet",
            false
        )
        .is_ok());
    }
    #[test]
    fn rejects_tampering_expiry_and_nonce_reuse() {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let v = sample();
        let sig = sk.sign(&canonical_bytes(&v));
        let mut tampered = v.clone();
        tampered.recipient = "attacker".into();
        assert_eq!(
            verify(
                &tampered,
                &sig.to_bytes(),
                sk.verifying_key().as_bytes(),
                150,
                "agentpay/solana-devnet",
                false
            ),
            Err(VerifyError::InvalidSignature)
        );
        assert_eq!(
            verify(
                &v,
                &sig.to_bytes(),
                sk.verifying_key().as_bytes(),
                250,
                "agentpay/solana-devnet",
                false
            ),
            Err(VerifyError::Expired)
        );
        assert_eq!(
            verify(
                &v,
                &sig.to_bytes(),
                sk.verifying_key().as_bytes(),
                150,
                "agentpay/solana-devnet",
                true
            ),
            Err(VerifyError::NonceUsed)
        );
    }
}
