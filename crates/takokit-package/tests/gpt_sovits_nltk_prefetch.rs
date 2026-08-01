const GPT_SOVITS_ADAPTER: &str =
    include_str!("../../../runners/python/gpt_sovits_adapter.py");

#[test]
fn gpt_sovits_prefetches_current_and_legacy_english_nltk_data() {
    assert!(GPT_SOVITS_ADAPTER.contains("prepare_nltk_resources"));
    assert!(GPT_SOVITS_ADAPTER.contains("averaged_perceptron_tagger_eng"));
    assert!(GPT_SOVITS_ADAPTER.contains("averaged_perceptron_tagger"));
    assert!(GPT_SOVITS_ADAPTER.contains("cmudict"));
    assert!(GPT_SOVITS_ADAPTER.contains("NLTK_DATA"));
    assert!(GPT_SOVITS_ADAPTER.contains("download_missing=True"));
    assert!(GPT_SOVITS_ADAPTER.contains("download_missing=False"));
}
