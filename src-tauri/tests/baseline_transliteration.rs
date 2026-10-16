use ruvox_tauri_lib::pipeline::normalizers::english::EnglishNormalizer;

#[test]
fn english_transliteration_baseline() {
    let content = std::fs::read_to_string("tests/fixtures/transliteration/baseline.csv").unwrap();
    let mut lines = content.lines();
    assert_eq!(lines.next().unwrap(), "word,current");
    let mut normalizer = EnglishNormalizer::default();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let (word, current) = line.split_once(',').unwrap();
        assert_eq!(
            normalizer.normalize(word, false),
            current,
            "baseline mismatch for {word}"
        );
    }
}
