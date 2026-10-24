/// Escapes dynamic text for Telegram's legacy Markdown parse mode. Titles,
/// genres or formats containing `_`, `*`, `[` or `` ` `` would otherwise make
/// the API reject the whole message.
pub fn escape_markdown(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(character, '\\' | '_' | '*' | '[' | ']' | '`') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::escape_markdown;

    #[test]
    fn escapes_legacy_markdown_characters() {
        assert_eq!(
            escape_markdown("A_B *C* [D] `E`"),
            "A\\_B \\*C\\* \\[D\\] \\`E\\`"
        );
        assert_eq!(
            escape_markdown("Vingadores: Ultimato"),
            "Vingadores: Ultimato"
        );
    }
}
