use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use mofa_orchestrator::marketplace::{Resolver, SignatureVerifier, TrustScorer};
use mofa_orchestrator::models::{PluginDependency, PluginManifest, Signature};

#[test]
fn resolver_validates_versions() {
    let manifest = PluginManifest {
        id: "plugin-a".into(),
        version: "1.2.3".into(),
        dependencies: vec![PluginDependency {
            id: "dep".into(),
            version_req: ">=1.0".into(),
        }],
        content_hash: "abc".into(),
        signature: None,
        metadata: serde_json::json!({}),
    };
    let resolver = Resolver::default();
    let deps = resolver.resolve(&manifest).unwrap();
    assert_eq!(deps.len(), 1);
}

#[test]
fn resolver_rejects_duplicate_deps() {
    let manifest = PluginManifest {
        id: "plugin-a".into(),
        version: "1.2.3".into(),
        dependencies: vec![
            PluginDependency {
                id: "dep".into(),
                version_req: ">=1.0".into(),
            },
            PluginDependency {
                id: "dep".into(),
                version_req: ">=1.0".into(),
            },
        ],
        content_hash: "abc".into(),
        signature: None,
        metadata: serde_json::json!({}),
    };
    let resolver = Resolver::default();
    let err = resolver.resolve(&manifest).unwrap_err();
    assert!(matches!(
        err,
        mofa_orchestrator::error::OrchestratorError::Dependency(_)
    ));
}

#[test]
fn signature_verifier_rejects_missing_signature() {
    let verifier = SignatureVerifier::default();
    let manifest = PluginManifest {
        id: "plugin-a".into(),
        version: "1.0.0".into(),
        dependencies: vec![],
        content_hash: "abc".into(),
        signature: None,
        metadata: serde_json::json!({}),
    };
    let key = SigningKey::from_bytes(&[1u8; 32]);
    let public = key.verifying_key();
    let err = verifier.verify(&manifest, &public).unwrap_err();
    assert!(matches!(
        err,
        mofa_orchestrator::error::OrchestratorError::Crypto(_)
    ));
}

#[test]
fn signature_verifier_accepts_valid_signature() {
    let key = SigningKey::from_bytes(&[2u8; 32]);
    let public = key.verifying_key();
    let content_hash = "abc".as_bytes();
    let sig = key.sign(content_hash);
    let sig_b64 = base64::prelude::BASE64_STANDARD.encode(sig.to_bytes());

    let manifest = PluginManifest {
        id: "plugin-a".into(),
        version: "1.0.0".into(),
        dependencies: vec![],
        content_hash: "abc".into(),
        signature: Some(Signature {
            public_key_id: "test-key".into(),
            signature_b64: sig_b64,
        }),
        metadata: serde_json::json!({}),
    };

    let verifier = SignatureVerifier::default();
    verifier.verify(&manifest, &public).unwrap();
}

#[test]
fn trust_scorer_combines_inputs() {
    let scorer = TrustScorer::default();
    let score_low = scorer.score(10, 0, 3.0);
    let score_high = scorer.score(10000, 5, 4.5);
    assert!(score_high > score_low);
}
