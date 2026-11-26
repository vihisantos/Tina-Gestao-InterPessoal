use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

#[derive(serde::Serialize)]
pub(crate) struct AuditPayload<'a> {
    pub id: &'a str,
    pub credential_id: &'a str,
    pub task_id: &'a str,
    pub requester_id: &'a str,
    pub delegator_id: Option<&'a str>,
    pub reviewer_id: Option<&'a str>,
    pub action: &'a str,
    pub resource_type: &'a str,
    pub resource_id: &'a str,
    pub outcome: &'a str,
    pub reason: Option<&'a str>,
    pub model_id: &'a str,
    pub model_version: &'a str,
    pub occurred_at: &'a str,
    pub nonce: &'a str,
    pub payload_digest: [u8; 32],
    pub previous_digest: Option<[u8; 32]>,
}

impl AuditPayload<'_> {
    pub(crate) fn digest(&self) -> Result<[u8; 32], serde_json::Error> {
        let mut hash = Sha256::new();
        hash.update(b"Tina.AgentAudit.v1\0");
        hash.update(serde_json::to_vec(self)?);
        Ok(hash.finalize().into())
    }
}

pub(crate) struct SignedEvent<'a> {
    pub payload: AuditPayload<'a>,
    pub digest: [u8; 32],
    pub signature: [u8; 64],
}

pub(crate) fn verify_chain(
    credential_id: &str,
    public_key: &[u8; 32],
    events: &[SignedEvent<'_>],
    expected_head: Option<[u8; 32]>,
) -> bool {
    let Ok(key) = VerifyingKey::from_bytes(public_key) else {
        return false;
    };
    let mut previous = None;
    let mut nonces = std::collections::HashSet::new();
    for event in events {
        if event.payload.credential_id != credential_id
            || event.payload.previous_digest != previous
            || !nonces.insert(event.payload.nonce)
            || event.payload.digest().ok() != Some(event.digest)
            || key
                .verify_strict(&event.digest, &Signature::from_bytes(&event.signature))
                .is_err()
        {
            return false;
        }
        previous = Some(event.digest);
    }
    previous == expected_head
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn rejects_tampering_and_truncation() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let public = key.verifying_key().to_bytes();
        let payload = AuditPayload {
            id: "event",
            credential_id: "credential",
            task_id: "task",
            requester_id: "owner",
            delegator_id: None,
            reviewer_id: None,
            action: "read",
            resource_type: "file",
            resource_id: "file-1",
            outcome: "executed",
            reason: None,
            model_id: "model",
            model_version: "1",
            occurred_at: "2026-11-26T00:00:00Z",
            nonce: "nonce",
            payload_digest: [0; 32],
            previous_digest: None,
        };
        let digest = payload.digest().unwrap();
        let mut events = [SignedEvent {
            payload,
            digest,
            signature: key.sign(&digest).to_bytes(),
        }];
        assert!(verify_chain("credential", &public, &events, Some(digest)));
        assert!(!verify_chain("credential", &public, &[], Some(digest)));
        events[0].payload.action = "delete";
        assert!(!verify_chain("credential", &public, &events, Some(digest)));
    }
}

/// Persiste somente eventos cuja assinatura e encadeamento são válidos.
/// O gatilho SQL rejeita bifurcações concorrentes e credenciais revogadas.
pub(crate) async fn append_verified(
    pool: &sqlx::SqlitePool,
    event: &SignedEvent<'_>,
) -> Result<(), sqlx::Error> {
    let p = &event.payload;
    let mut tx = pool.begin().await?;
    let public: Vec<u8> = sqlx::query_scalar(
        "SELECT public_key FROM agent_credentials WHERE id=? AND revoked_at IS NULL",
    )
    .bind(p.credential_id)
    .fetch_one(&mut *tx)
    .await?;
    let public: [u8; 32] = public.try_into().map_err(|_| {
        sqlx::Error::Protocol("invalid audit public key".into())
    })?;
    let valid = VerifyingKey::from_bytes(&public)
        .map(|key| {
            key.verify_strict(&event.digest, &Signature::from_bytes(&event.signature))
                .is_ok()
        })
        .unwrap_or(false);
    if !valid || p.digest().ok() != Some(event.digest) {
        return Err(sqlx::Error::Protocol("invalid audit signature".into()));
    }
    sqlx::query(
        "INSERT INTO agent_audit_events
        (id,credential_id,task_id,requester_id,delegator_id,reviewer_id,action,
        resource_type,resource_id,outcome,reason,model_id,model_version,occurred_at,
        nonce,payload_digest,previous_digest,event_digest,signature)
        VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(p.id)
    .bind(p.credential_id)
    .bind(p.task_id)
    .bind(p.requester_id)
    .bind(p.delegator_id)
    .bind(p.reviewer_id)
    .bind(p.action)
    .bind(p.resource_type)
    .bind(p.resource_id)
    .bind(p.outcome)
    .bind(p.reason)
    .bind(p.model_id)
    .bind(p.model_version)
    .bind(p.occurred_at)
    .bind(p.nonce)
    .bind(p.payload_digest.as_slice())
    .bind(p.previous_digest.as_ref().map(|v| v.as_slice()))
    .bind(event.digest.as_slice())
    .bind(event.signature.as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await
}
