use mofa_orchestrator::models::CapabilityDescriptor;
use mofa_orchestrator::registry::CapabilityRegistry;

fn cap(id: &str, name: &str, desc: &str, tags: &[&str]) -> CapabilityDescriptor {
    CapabilityDescriptor {
        id: id.into(),
        name: name.into(),
        description: desc.into(),
        tags: tags.iter().map(|t| t.to_string()).collect(),
        embeddings: None,
        trust_score: Some(0.8),
        availability: Some(0.9),
    }
}

#[test]
fn registry_search_scores_matches() {
    let mut reg = CapabilityRegistry::default();
    reg.publish(cap(
        "cap-a",
        "PDF Extract",
        "extract text",
        &["pdf", "extract"],
    ))
    .unwrap();
    reg.publish(cap("cap-b", "Image Tagger", "tags images", &["vision"]))
        .unwrap();

    let results = reg
        .search(
            "pdf",
            None,
            &mofa_orchestrator::registry::SearchConfig::default(),
        )
        .unwrap();
    assert_eq!(results.first().unwrap().id, "cap-a");
}
