pub fn accepted(short_name: &str) -> bool {
    let s = short_name.trim().to_lowercase();
    if s.starts_with("public domain") || s.starts_with("cc0") || s.starts_with("godl-india") {
        return true;
    }
    if s == "attribution" {
        return true;
    }
    if let Some(rest) = s.strip_prefix("cc by") {
        return rest.is_empty() || rest.starts_with(' ') || rest.starts_with("-sa");
    }
    false
}

#[cfg(test)]
mod tests {
    use super::accepted;

    #[test]
    fn accepts_allowlisted_families_with_versions() {
        for s in [
            "CC BY-SA 3.0 nl",
            "CC BY-SA 4.0",
            "CC BY 2.0",
            "CC BY",
            "cc by-sa 2.5",
            "CC0",
            "CC0 1.0",
            "Public domain",
            "Public Domain",
            "Attribution",
            "GODL-India",
        ] {
            assert!(accepted(s), "{s}");
        }
    }

    #[test]
    fn rejects_everything_else() {
        for s in [
            "Fair use",
            "CC BY-NC 2.0",
            "CC BY-NC-SA 3.0",
            "CC BY-ND 4.0",
            "Copyrighted free use",
            "Attribution-ShareAlike",
            "FAL",
            "",
        ] {
            assert!(!accepted(s), "{s}");
        }
    }
}
