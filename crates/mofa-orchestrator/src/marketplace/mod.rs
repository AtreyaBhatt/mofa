use crate::error::{OrchestratorError, OrchestratorResult};
use crate::models::{PluginDependency, PluginManifest};
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use semver::{Version, VersionReq};

#[derive(Debug, Default, Clone)]
pub struct Resolver;

impl Resolver {
    pub fn resolve(&self, manifest: &PluginManifest) -> OrchestratorResult<Vec<PluginDependency>> {
        // Placeholder: return deps after validating SemVer ranges.
        Version::parse(&manifest.version)
            .map_err(|e| OrchestratorError::Marketplace(format!("invalid version: {e}")))?;
        // Basic conflict detection: duplicate dependency IDs are rejected.
        let mut seen = std::collections::HashSet::new();
        for dep in &manifest.dependencies {
            if !seen.insert(dep.id.clone()) {
                return Err(OrchestratorError::Dependency(format!(
                    "duplicate dependency id: {}",
                    dep.id
                )));
            }
            VersionReq::parse(&dep.version_req).map_err(|e| {
                OrchestratorError::Marketplace(format!("invalid dep version req: {e}"))
            })?;
        }
        Ok(manifest.dependencies.clone())
    }
}

#[derive(Debug, Default, Clone)]
pub struct SignatureVerifier;

impl SignatureVerifier {
    pub fn verify(
        &self,
        manifest: &PluginManifest,
        public_key: &VerifyingKey,
    ) -> OrchestratorResult<()> {
        let sig = manifest
            .signature
            .as_ref()
            .ok_or_else(|| OrchestratorError::Crypto("missing signature".into()))?;
        let mut buf = Vec::new();
        base64::prelude::BASE64_STANDARD
            .decode_vec(&sig.signature_b64, &mut buf)
            .map_err(|e| OrchestratorError::Crypto(format!("invalid signature b64: {e}")))?;
        let arr: [u8; 64] = buf
            .try_into()
            .map_err(|_| OrchestratorError::Crypto("signature length invalid".into()))?;
        let signature = Signature::from_bytes(&arr);

        let msg = manifest.content_hash.as_bytes();
        public_key
            .verify_strict(msg, &signature)
            .map_err(|e| OrchestratorError::Crypto(format!("signature verification failed: {e}")))
    }
}

#[derive(Debug, Default, Clone)]
pub struct TrustScorer;

impl TrustScorer {
    pub fn score(&self, downloads: u64, audits_passed: u64, rating: f32) -> f32 {
        let dl_score = (downloads as f32).ln().max(0.0);
        let audit_score = audits_passed as f32;
        let rating_score = rating.clamp(0.0, 5.0);
        (0.4 * dl_score + 0.4 * audit_score + 0.2 * rating_score).min(100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{Resolver, SignatureVerifier, TrustScorer};
    use crate::models::{PluginDependency, PluginManifest};

    fn manifest_with_deps(deps: Vec<PluginDependency>) -> PluginManifest {
        PluginManifest {
            id: "m1".into(),
            version: "0.1.0".into(),
            dependencies: deps,
            content_hash: "hash".into(),
            signature: None,
            metadata: serde_json::Value::Null,
        }
    }

    #[test]
    fn resolver_rejects_duplicate_dep_ids() {
        let resolver = Resolver::default();
        let manifest = manifest_with_deps(vec![
            PluginDependency {
                id: "a".into(),
                version_req: "^1.0.0".into(),
            },
            PluginDependency {
                id: "a".into(),
                version_req: "^1.0.0".into(),
            },
        ]);
        let err = resolver.resolve(&manifest).unwrap_err();
        assert!(matches!(
            err,
            crate::error::OrchestratorError::Dependency(_)
        ));
    }

    #[test]
    fn trust_scorer_monotonic_with_downloads() {
        let scorer = TrustScorer::default();
        let low = scorer.score(1, 0, 0.0);
        let high = scorer.score(10_000, 0, 0.0);
        assert!(high > low);
    }

    #[test]
    fn signature_verifier_rejects_missing_signature() {
        let verifier = SignatureVerifier::default();
        let manifest = manifest_with_deps(vec![]);
        // Use a deterministic zeroed key for test; it will fail before using the key
        let pk_bytes = [0u8; 32];
        let key = ed25519_dalek::VerifyingKey::from_bytes(&pk_bytes)
            .unwrap_or_else(|_| ed25519_dalek::VerifyingKey::from_bytes(&[1u8; 32]).unwrap());
        let err = verifier.verify(&manifest, &key).unwrap_err();
        assert!(matches!(err, crate::error::OrchestratorError::Crypto(_)));
    }
}
