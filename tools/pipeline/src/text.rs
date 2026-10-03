use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

pub fn normalize(s: &str) -> String {
    let stripped: String = s
        .nfkd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase();
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn ascii_letters(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in normalize(s).chars() {
        match c {
            'a'..='z' | ' ' | '-' => out.push(c),
            'ł' => out.push('l'),
            'ø' | 'ɔ' => out.push('o'),
            'đ' | 'ð' => out.push('d'),
            'æ' => out.push_str("ae"),
            'œ' => out.push_str("oe"),
            'ß' => out.push_str("ss"),
            'þ' => out.push_str("th"),
            'ı' => out.push('i'),
            'ħ' => out.push('h'),
            'ŧ' => out.push('t'),
            _ => {}
        }
    }
    out
}

pub fn first_token(s: &str) -> Option<&str> {
    s.split_whitespace().next()
}

pub fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let decoded = decode_entities(&out);
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';') else {
            out.push_str(rest);
            return out;
        };
        let entity = &rest[1..end];
        let replacement = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            "nbsp" | "#160" => Some(' '),
            _ => entity
                .strip_prefix('#')
                .and_then(|n| {
                    n.strip_prefix('x')
                        .map(|h| u32::from_str_radix(h, 16).ok())
                        .unwrap_or_else(|| n.parse().ok())
                })
                .and_then(char::from_u32),
        };
        match replacement {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_drops_diacritics_and_case() {
        assert_eq!(normalize("Antonín"), "antonin");
        assert_eq!(normalize("ZOË"), "zoe");
        assert_eq!(normalize("Łukasz"), "łukasz");
        assert_eq!(normalize("François"), "francois");
    }

    #[test]
    fn normalize_collapses_whitespace_and_keeps_hyphen() {
        assert_eq!(normalize("  Jean-Paul   Marie "), "jean-paul marie");
        assert_eq!(normalize("Mary\tAnn"), "mary ann");
    }

    #[test]
    fn normalize_handles_compatibility_forms() {
        assert_eq!(normalize("ﬁona"), "fiona");
        assert_eq!(normalize("Ｊｏｈｎ"), "john");
    }

    #[test]
    fn ascii_letters_folds_what_nfkd_leaves_behind() {
        assert_eq!(ascii_letters("Łukasz"), "lukasz");
        assert_eq!(ascii_letters("Søren"), "soren");
        assert_eq!(ascii_letters("Þórður"), "thordur");
        assert_eq!(ascii_letters("Jean-Paul"), "jean-paul");
        assert_eq!(ascii_letters("Иван"), "");
        assert_eq!(ascii_letters("Ali2"), "ali");
    }

    #[test]
    fn strip_html_removes_tags_and_entities() {
        let s = r#"<a href="//commons.wikimedia.org/wiki/User:X" title="User:X">Jane &amp; Co</a>"#;
        assert_eq!(strip_html(s), "Jane & Co");
        assert_eq!(
            strip_html("<span>Bundesarchiv,</span><br/>Bild 183"),
            "Bundesarchiv, Bild 183"
        );
        assert_eq!(strip_html("A&#39;B &#x41; &nbsp;C"), "A'B A C");
        assert_eq!(strip_html("Rock & Roll"), "Rock & Roll");
    }
}
