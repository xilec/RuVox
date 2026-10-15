use ruvox_tauri_lib::pipeline::normalizers::english::transliterate_simple;

#[test]
fn english_transliteration_baseline() {
    let content = std::fs::read_to_string("tests/fixtures/transliteration/baseline.csv").unwrap();
    let mut lines = content.lines();
    assert_eq!(lines.next().unwrap(), "word,current");
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let (word, current) = line.split_once(',').unwrap();
        assert_eq!(
            transliterate_simple(word),
            current,
            "baseline mismatch for {word}"
        );
    }
}
