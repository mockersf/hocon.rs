use aho_corasick::AhoCorasick;
use lazy_static::lazy_static;
use std::borrow::Cow;
use std::ops::Range;

/// Unescape a JSON string
pub(crate) fn unescape(input: &str) -> Cow<str> {
    const PATTERNS: &[&str] = &[
        r#"\""#, r"\\", r"\/", r"\b", r"\f", r"\n", r"\r", r"\t", r"\u",
    ];
    const REPLACEMENTS: &[&str] = &["\"", "\\", "/", "\x08", "\x0c", "\x0a", "\x0d", "\x09"];
    const HIGH_SURROGATES: Range<u16> = 0xd800..0xdc00;
    const LOW_SURROGATES: Range<u16> = 0xdc00..0xe000;
    lazy_static! {
        static ref AC: AhoCorasick = AhoCorasick::new_auto_configured(PATTERNS);
    }

    let mut res = Cow::default();
    let mut last_start: usize = 0;
    let mut surrogates_vec: [u16; 2] = [0, 0];
    for mat in AC.find_iter(input) {
        res += &input[last_start..mat.start()];
        last_start = mat.end();

        if let Some(repl) = REPLACEMENTS.get(mat.pattern()) {
            res += *repl;
        } else if mat.end() + 4 <= input.len() {
            // Handle \u
            last_start += 4;
            let hex_digits = &input[mat.end()..mat.end() + 4];
            if let Ok(cp) = u16::from_str_radix(hex_digits, 16) {
                // Handle Unicode surrogate pairs
                if HIGH_SURROGATES.contains(&cp) {
                    // Beginning of surrogate pair
                    surrogates_vec[0] = cp;
                } else {
                    surrogates_vec[1] = cp;
                    let surrogates_vec_ref = if LOW_SURROGATES.contains(&cp) {
                        // Ending of surrogate pair, call: from_utf16([high, low])
                        &surrogates_vec
                    } else {
                        // Not a surrogate pair, call: from_utf16([cp])
                        &surrogates_vec[1..]
                    };
                    if let Ok(str) = String::from_utf16(surrogates_vec_ref) {
                        res += Cow::from(str);
                    }
                }
            }
        }
    }
    res += &input[last_start..];
    res
}
