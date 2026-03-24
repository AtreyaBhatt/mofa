//! BM25 helper kept behind the `bm25` feature.

pub(crate) fn avg_doc_len(corpus: &[String]) -> f32 {
    if corpus.is_empty() {
        return 1.0;
    }
    let total: usize = corpus.iter().map(|d| d.split_whitespace().count()).sum();
    (total as f32 / corpus.len() as f32).max(1.0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn avg_doc_len_handles_empty() {
        assert_eq!(super::avg_doc_len(&[]), 1.0);
    }

    #[test]
    fn avg_doc_len_computes_mean() {
        let docs = vec!["a b".to_string(), "a b c d".to_string()];
        let avg = super::avg_doc_len(&docs);
        assert!((avg - 3.0).abs() < f32::EPSILON);
    }
}
