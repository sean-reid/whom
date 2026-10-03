// Commons reports a short name like "CC BY-SA 3.0 nl"; a family token list may be
// followed by one version and one jurisdiction, nothing else.
pub fn accepted(short_name: &str) -> bool {
    let lowered = short_name.trim().to_lowercase().replace('-', " ");
    let tokens: Vec<&str> = lowered.split_whitespace().collect();
    let rest = match tokens.as_slice() {
        ["public", "domain", rest @ ..]
        | ["cc0", rest @ ..]
        | ["cc", "by", "sa", rest @ ..]
        | ["cc", "by", rest @ ..]
        | ["attribution", rest @ ..]
        | ["godl", "india", rest @ ..] => rest,
        _ => return false,
    };
    match rest {
        [] => true,
        [version] => is_version(version),
        [version, jurisdiction] => is_version(version) && is_jurisdiction(jurisdiction),
        _ => false,
    }
}

fn is_version(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_digit()) && s.chars().all(|c| c.is_ascii_digit() || c == '.')
}

fn is_jurisdiction(s: &str) -> bool {
    (2..=3).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::accepted;

    #[test]
    fn accepts_allowlisted_families_with_versions() {
        for s in [
            "CC BY-SA 3.0 nl",
            "CC BY-SA 4.0",
            "CC-BY-SA 4.0",
            "cc-by-sa-2.5-de",
            "CC BY 2.0",
            "CC BY",
            "cc by-sa 2.5",
            "CC BY-SA 2.5 de",
            "CC0",
            "CC0 1.0",
            "Public domain",
            "Public Domain",
            "public domain 1.0",
            "Attribution",
            "Attribution 3.0",
            "GODL-India",
            "GODL-India 1.0",
        ] {
            assert!(accepted(s), "{s}");
        }
    }

    #[test]
    fn rejects_everything_else() {
        for s in [
            "Fair use",
            "CC BY-SA-NC",
            "CC BY-NC 2.0",
            "CC BY-NC 4.0",
            "CC BY-NC-SA 3.0",
            "CC BY-ND",
            "CC BY-ND 4.0",
            "CC BY-SA 4.0 nl extra",
            "CC BY-SA nl",
            "CC BY-SA 4.0 netherlands",
            "CC BY-SA v4",
            "Copyrighted free use",
            "Attribution-ShareAlike",
            "Public domain mark",
            "FAL",
            "",
        ] {
            assert!(!accepted(s), "{s}");
        }
    }
}
