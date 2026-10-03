use crate::families;
use crate::store::Name;
use crate::text::{ascii_letters, is_name_label, normalize};
use rphonetic::DoubleMetaphone;
use std::collections::{BTreeMap, BTreeSet};

pub const MIN_ERA_SAMPLE: u64 = 5;

pub struct GivenName {
    pub qid: String,
    pub label: String,
    pub langs: Vec<String>,
    pub count: u64,
    pub same_as: Vec<String>,
    pub hypocorism: bool,
}

pub struct Graph {
    pub names: BTreeMap<String, Name>,
    pub langs_seen: BTreeSet<String>,
    pub langs_without_family: BTreeSet<String>,
    pub unlabelled: usize,
    pub rejected_labels: usize,
    pub key_of_qid: BTreeMap<String, String>,
    links: BTreeMap<String, BTreeSet<String>>,
    hypocorisms: BTreeSet<String>,
    years: BTreeMap<String, BTreeMap<i32, u64>>,
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
            rejected_labels: 0,
            key_of_qid: BTreeMap::new(),
            links: BTreeMap::new(),
            hypocorisms: BTreeSet::new(),
            years: BTreeMap::new(),
            dm: DoubleMetaphone::new(None),
        }
    }

    pub fn add_given_name(&mut self, gn: &GivenName) {
        let label = gn.label.trim();
        if label.is_empty() {
            self.unlabelled += 1;
            return;
        }
        if !is_name_label(label) {
            self.rejected_labels += 1;
            return;
        }
        let key = normalize(label);
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
        self.key_of_qid.insert(gn.qid.clone(), key.clone());
        self.links
            .entry(key.clone())
            .or_default()
            .extend(gn.same_as.iter().cloned());
        if gn.hypocorism {
            self.hypocorisms.insert(key.clone());
        }
        let (dm, rhyme) = self.codes(&key);
        let entry = self.names.entry(key).or_insert_with(|| Name {
            display: gn.label.trim().to_string(),
            langs: Vec::new(),
            families: Vec::new(),
            count: 0,
            dm,
            rhyme,
            era: None,
            same_as: Vec::new(),
            short_of: Vec::new(),
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
        if !is_name_label(display.trim()) {
            return;
        }
        let key = normalize(display);
        if self.names.contains_key(&key) {
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
                era: None,
                same_as: Vec::new(),
                short_of: Vec::new(),
            },
        );
    }

    pub fn add_birth_years(&mut self, qid: &str, year: i32, count: u64) {
        let Some(key) = self.key_of_qid.get(qid) else {
            return;
        };
        *self
            .years
            .entry(key.clone())
            .or_default()
            .entry(year)
            .or_insert(0) += count;
    }

    // Resolves P460 links to keys in both directions and derives short forms;
    // call once after every given name is added.
    pub fn link(&mut self) {
        let mut same: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (key, targets) in &self.links {
            for t in targets {
                let Some(other) = self.key_of_qid.get(t) else {
                    continue;
                };
                if other == key || !self.names.contains_key(other) {
                    continue;
                }
                same.entry(key.clone()).or_default().insert(other.clone());
                same.entry(other.clone()).or_default().insert(key.clone());
            }
        }
        for (key, others) in &same {
            let langs: BTreeSet<&str> = self.names[key].langs.iter().map(String::as_str).collect();
            let short_of: Vec<String> = if self.hypocorisms.contains(key) {
                others
                    .iter()
                    .filter(|o| !self.hypocorisms.contains(*o))
                    .filter(|o| {
                        let theirs = &self.names[*o].langs;
                        langs.is_empty()
                            || theirs.is_empty()
                            || theirs.iter().any(|l| langs.contains(l.as_str()))
                    })
                    .cloned()
                    .collect()
            } else {
                Vec::new()
            };
            let entry = self.names.get_mut(key).expect("linked key exists");
            entry.same_as = others.iter().cloned().collect();
            entry.short_of = short_of;
        }
        for (key, dist) in &self.years {
            if let Some(entry) = self.names.get_mut(key) {
                entry.era = median_year(dist);
            }
        }
    }

    fn codes(&self, normalized: &str) -> (String, String) {
        let ascii = ascii_letters(normalized);
        let dm = if ascii.is_empty() {
            String::new()
        } else {
            self.dm.double_metaphone(&ascii).primary()
        };
        let rhyme = rhyme_of(&dm);
        (dm, rhyme)
    }
}

pub fn median_year(dist: &BTreeMap<i32, u64>) -> Option<i32> {
    let total: u64 = dist.values().sum();
    if total < MIN_ERA_SAMPLE {
        return None;
    }
    let lower_rank = total.div_ceil(2);
    let upper_rank = total / 2 + 1;
    let mut seen = 0;
    let mut lower = None;
    for (year, n) in dist {
        seen += n;
        if lower.is_none() && seen >= lower_rank {
            lower = Some(*year);
        }
        if seen >= upper_rank {
            let lo = lower.unwrap_or(*year);
            return Some((lo + *year).div_euclid(2));
        }
    }
    lower
}

pub fn rhyme_of(dm: &str) -> String {
    let n = dm.chars().count();
    dm.chars().skip(n.saturating_sub(2)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gn(qid: &str, label: &str, langs: &[&str], count: u64) -> GivenName {
        GivenName {
            qid: qid.into(),
            label: label.into(),
            langs: langs.iter().map(|s| s.to_string()).collect(),
            count,
            same_as: Vec::new(),
            hypocorism: false,
        }
    }

    fn dist(pairs: &[(i32, u64)]) -> BTreeMap<i32, u64> {
        pairs.iter().cloned().collect()
    }

    #[test]
    fn median_year_needs_five_people_and_splits_even_counts() {
        assert_eq!(median_year(&dist(&[(1950, 4)])), None);
        assert_eq!(median_year(&dist(&[(1950, 5)])), Some(1950));
        assert_eq!(
            median_year(&dist(&[(1900, 2), (1950, 1), (2000, 2)])),
            Some(1950)
        );
        assert_eq!(median_year(&dist(&[(1900, 3), (1960, 3)])), Some(1930));
        assert_eq!(
            median_year(&dist(&[(1900, 3), (1960, 2), (1961, 1)])),
            Some(1930)
        );
        assert_eq!(median_year(&dist(&[(1900, 1), (1980, 10)])), Some(1980));
        assert_eq!(median_year(&dist(&[(-50, 3), (-40, 3)])), Some(-45));
    }

    #[test]
    fn same_as_links_both_ways_and_only_to_known_names() {
        let mut g = Graph::new();
        let mut bill = gn("Q10", "Bill", &["Q1860"], 1000);
        bill.same_as = vec!["Q20".into(), "Q30".into(), "Q99".into()];
        bill.hypocorism = true;
        g.add_given_name(&bill);
        g.add_given_name(&gn("Q20", "William", &["Q1860", "Q7411"], 5000));
        g.add_given_name(&gn("Q30", "Wilhelm", &["Q188"], 3000));
        let mut will = gn("Q40", "Will", &["Q1860"], 400);
        will.same_as = vec!["Q10".into()];
        will.hypocorism = true;
        g.add_given_name(&will);
        g.link();
        assert_eq!(g.names["bill"].same_as, vec!["wilhelm", "will", "william"]);
        assert_eq!(g.names["william"].same_as, vec!["bill"]);
        assert_eq!(g.names["wilhelm"].same_as, vec!["bill"]);
        assert_eq!(g.names["will"].same_as, vec!["bill"]);
        assert_eq!(g.names["bill"].short_of, vec!["william"]);
        assert!(g.names["will"].short_of.is_empty());
        assert!(g.names["william"].short_of.is_empty());
    }

    #[test]
    fn link_targets_resolve_through_the_normalized_label() {
        let mut g = Graph::new();
        let mut bob = gn("Q1", "Bob", &["Q1860"], 100);
        bob.same_as = vec!["Q2".into(), "Q3".into()];
        bob.hypocorism = true;
        g.add_given_name(&bob);
        g.add_given_name(&gn("Q2", "Róbert", &["Q9067"], 50));
        g.add_given_name(&gn("Q3", "Robert", &["Q1860"], 90));
        g.link();
        assert_eq!(g.names["bob"].same_as, vec!["robert"]);
        assert_eq!(g.names["bob"].short_of, vec!["robert"]);
        assert_eq!(g.names["robert"].same_as, vec!["bob"]);
    }

    #[test]
    fn birth_years_merge_across_items_sharing_a_key() {
        let mut g = Graph::new();
        g.add_given_name(&gn("Q1", "Jean", &[], 3));
        g.add_given_name(&gn("Q2", "Jean", &[], 3));
        g.add_birth_years("Q1", 1900, 3);
        g.add_birth_years("Q2", 1980, 3);
        g.add_birth_years("Q404", 1700, 50);
        g.link();
        assert_eq!(g.names["jean"].era, Some(1940));
    }

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
        assert_eq!(g.codes("łukasz"), g.codes("lukasz"));
        assert_eq!(g.codes("иван"), (String::new(), String::new()));
    }

    #[test]
    fn same_label_merges_items_and_ignores_multiple_languages() {
        let mut g = Graph::new();
        g.add_given_name(&gn(
            "Q1",
            "Jean",
            &["Q150", families::MULTIPLE_LANGUAGES],
            100,
        ));
        g.add_given_name(&gn("Q2", "Jean", &["Q1860", "Q150"], 60));
        let n = &g.names["jean"];
        assert_eq!(n.count, 160);
        assert_eq!(n.langs, vec!["Q150", "Q1860"]);
        assert_eq!(n.families, vec!["germanic", "romance"]);
        assert!(!g.langs_seen.contains(families::MULTIPLE_LANGUAGES));
    }

    #[test]
    fn unknown_language_is_reported_and_adds_no_family() {
        let mut g = Graph::new();
        g.add_given_name(&gn("Q1", "Xyz", &["Q999999999"], 51));
        assert!(g.names["xyz"].families.is_empty());
        assert!(g.langs_without_family.contains("Q999999999"));
    }

    #[test]
    fn labels_that_are_not_names_are_dropped_and_counted() {
        let mut g = Graph::new();
        g.add_given_name(&gn("Q1", ".", &[], 300));
        g.add_given_name(&gn("Q2", "\"Nastya\", \"Nastas\", or \"Nastenka", &[], 60));
        g.add_given_name(&gn("Q3", "Nastya", &[], 60));
        g.ensure_form("Bob (comics)");
        assert_eq!(g.rejected_labels, 2);
        assert_eq!(g.names.keys().collect::<Vec<_>>(), vec!["nastya"]);
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
