use crate::families;
use crate::store::Name;
use crate::text::normalize;
use rphonetic::DoubleMetaphone;
use std::collections::{BTreeMap, BTreeSet};

pub struct GivenName {
    pub label: String,
    pub langs: Vec<String>,
    pub count: u64,
}

pub struct Graph {
    pub names: BTreeMap<String, Name>,
    pub langs_seen: BTreeSet<String>,
    pub langs_without_family: BTreeSet<String>,
    pub unlabelled: usize,
    dm: DoubleMetaphone,
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            names: BTreeMap::new(),
            langs_seen: BTreeSet::new(),
            langs_without_family: BTreeSet::new(),
            unlabelled: 0,
            dm: DoubleMetaphone::new(None),
        }
    }

    pub fn add_given_name(&mut self, gn: &GivenName) {
        let key = normalize(&gn.label);
        if key.is_empty() {
            self.unlabelled += 1;
            return;
        }
        let langs: Vec<&str> = gn
            .langs
            .iter()
            .map(String::as_str)
            .filter(|l| *l != families::MULTIPLE_LANGUAGES)
            .collect();
        for l in &langs {
            self.langs_seen.insert(l.to_string());
            if families::family(l).is_none() {
                self.langs_without_family.insert(l.to_string());
            }
        }
        let (dm, rhyme) = self.codes(&key);
        let entry = self.names.entry(key).or_insert_with(|| Name {
            display: gn.label.trim().to_string(),
            langs: Vec::new(),
            families: Vec::new(),
            count: 0,
            dm,
            rhyme,
        });
        entry.count += gn.count;
        for l in langs {
            if !entry.langs.iter().any(|x| x == l) {
                entry.langs.push(l.to_string());
            }
            if let Some(f) = families::family(l) {
                if !entry.families.iter().any(|x| x == f) {
                    entry.families.push(f.to_string());
                }
            }
        }
        entry.langs.sort_by_key(|q| crate::store::qid_order(q));
        entry.families.sort();
    }

    pub fn ensure_form(&mut self, display: &str) {
        let key = normalize(display);
        if key.is_empty() || self.names.contains_key(&key) {
            return;
        }
        let (dm, rhyme) = self.codes(&key);
        self.names.insert(
            key,
            Name {
                display: display.trim().to_string(),
                langs: Vec::new(),
                families: Vec::new(),
                count: 0,
                dm,
                rhyme,
            },
        );
    }

    fn codes(&self, normalized: &str) -> (String, String) {
        let dm = self.dm.double_metaphone(normalized).primary();
        let rhyme = rhyme_of(&dm);
        (dm, rhyme)
    }
}

pub fn rhyme_of(dm: &str) -> String {
    let n = dm.chars().count();
    dm.chars().skip(n.saturating_sub(2)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rhyme_is_last_two_chars_or_whole_code() {
        assert_eq!(rhyme_of("ALKSNTR"), "TR");
        assert_eq!(rhyme_of("JN"), "JN");
        assert_eq!(rhyme_of("A"), "A");
        assert_eq!(rhyme_of(""), "");
    }

    #[test]
    fn double_metaphone_primary_is_stable() {
        let g = Graph::new();
        let (dm, rhyme) = g.codes("alexander");
        assert_eq!(dm, "ALKSNTR");
        assert_eq!(rhyme, "TR");
        let (dm, _) = g.codes("john");
        assert_eq!(dm, "JN");
    }

    #[test]
    fn same_label_merges_items_and_ignores_multiple_languages() {
        let mut g = Graph::new();
        g.add_given_name(&GivenName {
            label: "Jean".into(),
            langs: vec!["Q150".into(), families::MULTIPLE_LANGUAGES.into()],
            count: 100,
        });
        g.add_given_name(&GivenName {
            label: "Jean".into(),
            langs: vec!["Q1860".into(), "Q150".into()],
            count: 60,
        });
        let n = &g.names["jean"];
        assert_eq!(n.count, 160);
        assert_eq!(n.langs, vec!["Q150", "Q1860"]);
        assert_eq!(n.families, vec!["germanic", "romance"]);
        assert!(!g.langs_seen.contains(families::MULTIPLE_LANGUAGES));
    }

    #[test]
    fn unknown_language_is_reported_and_adds_no_family() {
        let mut g = Graph::new();
        g.add_given_name(&GivenName {
            label: "Xyz".into(),
            langs: vec!["Q999999999".into()],
            count: 51,
        });
        assert!(g.names["xyz"].families.is_empty());
        assert!(g.langs_without_family.contains("Q999999999"));
    }

    #[test]
    fn ensure_form_adds_a_zero_count_record_once() {
        let mut g = Graph::new();
        g.ensure_form("Bobby");
        g.ensure_form("bobby ");
        assert_eq!(g.names.len(), 1);
        assert_eq!(g.names["bobby"].count, 0);
        assert_eq!(g.names["bobby"].display, "Bobby");
    }
}
