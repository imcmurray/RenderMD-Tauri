//! Image-reference helpers: locating an image's markdown/HTML form in the
//! document, rebuilding its markup after a resize/alt edit, and offset
//! conversions between Rust byte offsets, char offsets, and the UTF-16
//! code-unit offsets used by CodeMirror positions.

use crate::render::html_escape;

pub fn char_to_byte_offset(text: &str, char_offset: usize) -> usize {
    text.char_indices()
        .nth(char_offset)
        .map(|(b, _)| b)
        .unwrap_or(text.len())
}

/// Convert a byte offset in `text` to a UTF-16 code-unit offset (CodeMirror
/// positions).
///
/// If `byte` lands in the middle of a multi-byte character, the offset is
/// floored to that character's start. Offsets at or past the end of `text`
/// return the total UTF-16 length.
pub fn byte_to_utf16_offset(text: &str, byte: usize) -> usize {
    let mut utf16 = 0usize;
    for (start, ch) in text.char_indices() {
        // `byte` is at this char's boundary, or inside it (mid-char →
        // floor to the char start): stop before counting this char.
        if byte < start + ch.len_utf8() {
            return utf16;
        }
        utf16 += ch.len_utf16();
    }
    utf16
}

/// Inverse of [`byte_to_utf16_offset`].
///
/// If `utf16` lands between the two code units of a surrogate pair, the
/// offset is floored to that character's start. Offsets at or past the end
/// of `text` return `text.len()`.
pub fn utf16_to_byte_offset(text: &str, utf16: usize) -> usize {
    let mut u = 0usize;
    for (start, ch) in text.char_indices() {
        // `utf16` is at this char's boundary, or splits its surrogate pair
        // (mid-char → floor to the char start): stop at this char's start.
        if utf16 < u + ch.len_utf16() {
            return start;
        }
        u += ch.len_utf16();
    }
    text.len()
}

// Locate the markdown or HTML form of an image with the given src in the
// buffer text. Returns (char_offset, char_len) for the *whole* image
// expression so the caller can replace it cleanly. Naive: matches the
// first occurrence; if the same src appears more than once in the doc,
// only the first is touched. Good enough for typical use.
pub fn find_image_ref(text: &str, src: &str) -> Option<(usize, usize)> {
    // Markdown form: ![alt](src) or ![alt](src "title").
    let needle = format!("({src}");
    if let Some(byte_idx) = text.find(&needle) {
        // Confirm the byte just before is `]` and walk back to find `![`.
        let before = &text[..byte_idx];
        if before.ends_with(']') {
            if let Some(bracket_byte) = before.rfind("![") {
                // Find the closing `)` after byte_idx.
                let after_paren_start = byte_idx + 1;
                if let Some(rel_close) = text[after_paren_start..].find(')') {
                    let end_byte = after_paren_start + rel_close + 1;
                    let char_start = text[..bracket_byte].chars().count();
                    let char_end = text[..end_byte].chars().count();
                    return Some((char_start, char_end - char_start));
                }
            }
        }
    }
    // HTML form: <img ... src="src" ...> or src='src'.
    for delim in ['"', '\''] {
        let needle = format!("src={delim}{src}{delim}");
        if let Some(byte_idx) = text.find(&needle) {
            let before = &text[..byte_idx];
            if let Some(img_byte) = before.rfind("<img") {
                if let Some(rel_close) = text[img_byte..].find('>') {
                    let end_byte = img_byte + rel_close + 1;
                    let char_start = text[..img_byte].chars().count();
                    let char_end = text[..end_byte].chars().count();
                    return Some((char_start, char_end - char_start));
                }
            }
        }
    }
    None
}

// Build the new markup for an image, preferring markdown form when no
// width is set so the doc stays clean. Width forces the HTML form since
// CommonMark doesn't have an inline width syntax.
pub fn build_image_markup(src: &str, width: Option<&str>, alt: &str) -> String {
    match width {
        None => format!("![{alt}]({src})"),
        Some(w) => {
            let alt_attr = if alt.is_empty() {
                String::new()
            } else {
                format!(" alt=\"{}\"", html_escape(alt))
            };
            format!(
                "<img src=\"{}\"{} width=\"{}\">",
                html_escape(src),
                alt_attr,
                html_escape(w)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- char_to_byte_offset ------------------------------------------------

    #[test]
    fn char_to_byte_ascii_and_multibyte() {
        assert_eq!(char_to_byte_offset("abc", 0), 0);
        assert_eq!(char_to_byte_offset("abc", 2), 2);
        // Past the end clamps to len.
        assert_eq!(char_to_byte_offset("abc", 10), 3);
        // "é" is 2 bytes.
        assert_eq!(char_to_byte_offset("éx", 1), 2);
    }

    // ---- byte <-> utf16 -----------------------------------------------------

    #[test]
    fn utf16_ascii_is_identity() {
        let s = "hello world";
        for b in 0..=s.len() {
            assert_eq!(byte_to_utf16_offset(s, b), b);
            assert_eq!(utf16_to_byte_offset(s, b), b);
        }
    }

    #[test]
    fn utf16_cjk_three_bytes_one_unit() {
        // Each CJK char here: 3 bytes in UTF-8, 1 code unit in UTF-16.
        let s = "你好x";
        assert_eq!(byte_to_utf16_offset(s, 0), 0);
        assert_eq!(byte_to_utf16_offset(s, 3), 1);
        assert_eq!(byte_to_utf16_offset(s, 6), 2);
        assert_eq!(byte_to_utf16_offset(s, 7), 3); // end
        assert_eq!(utf16_to_byte_offset(s, 0), 0);
        assert_eq!(utf16_to_byte_offset(s, 1), 3);
        assert_eq!(utf16_to_byte_offset(s, 2), 6);
        assert_eq!(utf16_to_byte_offset(s, 3), 7); // end
    }

    #[test]
    fn utf16_astral_emoji_surrogate_pair() {
        // 👍 U+1F44D: 4 bytes in UTF-8, 2 code units (surrogate pair) in UTF-16.
        let s = "a👍b";
        assert_eq!(byte_to_utf16_offset(s, 0), 0);
        assert_eq!(byte_to_utf16_offset(s, 1), 1); // start of emoji
        assert_eq!(byte_to_utf16_offset(s, 5), 3); // after emoji
        assert_eq!(byte_to_utf16_offset(s, 6), 4); // end
        assert_eq!(utf16_to_byte_offset(s, 0), 0);
        assert_eq!(utf16_to_byte_offset(s, 1), 1);
        assert_eq!(utf16_to_byte_offset(s, 3), 5);
        assert_eq!(utf16_to_byte_offset(s, 4), 6); // end
    }

    #[test]
    fn utf16_zwj_flag_sequence() {
        // 🏳️‍🌈 = U+1F3F3 U+FE0F U+200D U+1F308:
        //   UTF-8 bytes:  4 + 3 + 3 + 4 = 14
        //   UTF-16 units: 2 + 1 + 1 + 2 = 6
        let s = "🏳️‍🌈!";
        assert_eq!(s.len(), 15);
        assert_eq!(byte_to_utf16_offset(s, 0), 0);
        assert_eq!(byte_to_utf16_offset(s, 4), 2); // after U+1F3F3
        assert_eq!(byte_to_utf16_offset(s, 7), 3); // after U+FE0F
        assert_eq!(byte_to_utf16_offset(s, 10), 4); // after U+200D
        assert_eq!(byte_to_utf16_offset(s, 14), 6); // after U+1F308
        assert_eq!(byte_to_utf16_offset(s, 15), 7); // end
        assert_eq!(utf16_to_byte_offset(s, 2), 4);
        assert_eq!(utf16_to_byte_offset(s, 3), 7);
        assert_eq!(utf16_to_byte_offset(s, 4), 10);
        assert_eq!(utf16_to_byte_offset(s, 6), 14);
        assert_eq!(utf16_to_byte_offset(s, 7), 15); // end
    }

    #[test]
    fn utf16_mid_char_floors_to_char_start() {
        let s = "a👍b"; // emoji occupies bytes 1..5, utf16 units 1..3
                        // Byte offsets 2, 3, 4 are inside the emoji → floor to its start (1).
        for b in 2..5 {
            assert_eq!(byte_to_utf16_offset(s, b), 1);
        }
        // UTF-16 offset 2 splits the surrogate pair → floor to byte 1.
        assert_eq!(utf16_to_byte_offset(s, 2), 1);
    }

    #[test]
    fn utf16_offsets_past_end_clamp() {
        let s = "hé👍";
        assert_eq!(byte_to_utf16_offset(s, 100), 4); // 1 + 1 + 2 units
        assert_eq!(utf16_to_byte_offset(s, 100), s.len());
        assert_eq!(byte_to_utf16_offset("", 0), 0);
        assert_eq!(utf16_to_byte_offset("", 0), 0);
    }

    #[test]
    fn utf16_round_trips_at_char_boundaries() {
        let s = "a你👍🏳️‍🌈é!";
        for (byte, _) in s.char_indices() {
            let u = byte_to_utf16_offset(s, byte);
            assert_eq!(utf16_to_byte_offset(s, u), byte, "byte {byte} in {s:?}");
        }
        let end_u = byte_to_utf16_offset(s, s.len());
        assert_eq!(utf16_to_byte_offset(s, end_u), s.len());
    }

    // ---- find_image_ref / build_image_markup --------------------------------

    #[test]
    fn find_image_ref_markdown_form() {
        let text = "before ![alt text](img.png) after";
        let (start, len) = find_image_ref(text, "img.png").unwrap();
        assert_eq!(start, 7);
        assert_eq!(len, "![alt text](img.png)".chars().count());
    }

    #[test]
    fn find_image_ref_markdown_form_with_title() {
        let text = "![a](img.png \"title\")";
        let (start, len) = find_image_ref(text, "img.png").unwrap();
        assert_eq!(start, 0);
        assert_eq!(len, text.chars().count());
    }

    #[test]
    fn find_image_ref_html_form_both_quotes() {
        for text in [
            r#"x <img src="pic.jpg" width="80"> y"#,
            r#"x <img src='pic.jpg' width='80'> y"#,
        ] {
            let (start, len) = find_image_ref(text, "pic.jpg").unwrap();
            assert_eq!(start, 2, "in {text}");
            assert_eq!(len, text.chars().count() - 4, "in {text}");
        }
    }

    #[test]
    fn find_image_ref_counts_chars_not_bytes() {
        // Multibyte text before the image: offsets must be char-based.
        let text = "héllo 👍 ![a](img.png)";
        let (start, len) = find_image_ref(text, "img.png").unwrap();
        assert_eq!(start, "héllo 👍 ".chars().count());
        assert_eq!(len, "![a](img.png)".chars().count());
    }

    #[test]
    fn find_image_ref_missing_returns_none() {
        assert!(find_image_ref("no images here", "img.png").is_none());
        // src present but not in an image form.
        assert!(find_image_ref("see (img.png)", "img.png").is_none());
    }

    #[test]
    fn build_image_markup_prefers_markdown_without_width() {
        assert_eq!(
            build_image_markup("img.png", None, "alt"),
            "![alt](img.png)"
        );
    }

    #[test]
    fn build_image_markup_html_when_width_set() {
        assert_eq!(
            build_image_markup("img.png", Some("120"), "an \"alt\""),
            r#"<img src="img.png" alt="an &quot;alt&quot;" width="120">"#
        );
        // Empty alt drops the attribute entirely.
        assert_eq!(
            build_image_markup("img.png", Some("120"), ""),
            r#"<img src="img.png" width="120">"#
        );
    }
}
