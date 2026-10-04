use crate::families;
use crate::regions;
use crate::run::title_case;
use crate::store::Name;
use crate::text::{ascii_letters, keeps_form, normalize};
use rphonetic::DoubleMetaphone;
use std::collections::{BTreeMap, BTreeSet};
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

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
    display_holders: BTreeMap<String, u64>,
    links: BTreeMap<String, BTreeSet<String>>,
    hypocorisms: BTreeSet<String>,
    years: BTreeMap<String, BTreeMap<i32, u64>>,
    citizenships: BTreeMap<String, BTreeMap<&'static str, u64>>,
    pub unmapped_countries: BTreeMap<String, u64>,
    dm: DoubleMetaphone,
}

#[derive(Debug, Default, PartialEq)]
pub struct RegionStats {
    pub names_with_region: usize,
    pub regions: BTreeMap<String, String>,
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
            display_holders: BTreeMap::new(),
            links: BTreeMap::new(),
            hypocorisms: BTreeSet::new(),
            years: BTreeMap::new(),
            citizenships: BTreeMap::new(),
            unmapped_countries: BTreeMap::new(),
            dm: DoubleMetaphone::new(None),
        }
    }

    pub fn add_given_name(&mut self, gn: &GivenName) {
        let label = gn.label.trim();
        if label.is_empty() {
            self.unlabelled += 1;
            return;
        }
        if !keeps_form(label) {
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
        let holders = self.display_holders.entry(key.clone()).or_insert(0);
        let entry = self.names.entry(key).or_insert_with(|| Name {
            display: cased(gn.label.trim()),
            langs: Vec::new(),
            families: Vec::new(),
            count: 0,
            dm,
            rhyme,
            era: None,
            same_as: Vec::new(),
            short_of: Vec::new(),
            region: None,
            continent: None,
            region_share: None,
        });
        entry.count += gn.count;
        if label != entry.display && outranks(label, gn.count, &entry.display, *holders) {
            entry.display = cased(label);
        }
        *holders = (*holders).max(gn.count);
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
}

// When several given-name items share a key, the display is the label with
// the most holders; ties go to fewer accents, then alphabetical order.
fn outranks(label: &str, count: u64, current: &str, current_count: u64) -> bool {
    let marks = |s: &str| s.nfkd().filter(|c| is_combining_mark(*c)).count();
    (
        count,
        std::cmp::Reverse(marks(label)),
        std::cmp::Reverse(label),
    ) > (
        current_count,
        std::cmp::Reverse(marks(current)),
        std::cmp::Reverse(current),
    )
}

// Wikidata labels are sometimes stored lowercase; a name shows with a capital.
fn cased(label: &str) -> String {
    if label.chars().next().is_some_and(char::is_lowercase) {
        title_case(label)
    } else {
        label.to_string()
    }
}

impl Graph {
    pub fn ensure_form(&mut self, display: &str) {
        if !keeps_form(display) {
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
                display: cased(display.trim()),
                langs: Vec::new(),
                families: Vec::new(),
                count: 0,
                dm,
                rhyme,
                era: None,
                same_as: Vec::new(),
                short_of: Vec::new(),
                region: None,
                continent: None,
                region_share: None,
            },
        );
    }

    pub fn add_citizenship(&mut self, qid: &str, country: &str, count: u64) {
        let Some(key) = self.key_of_qid.get(qid).cloned() else {
            return;
        };
        self.add_citizenship_to_key(&key, regions::region_of_qid(country), country, count);
    }

    // Pool records name a country, not a qid; the label resolves through the table.
    pub fn add_citizenship_label(&mut self, key: &str, label: &str) {
        if self.names.contains_key(key) {
            self.add_citizenship_to_key(key, regions::region_of_label(label), label, 1);
        }
    }

    fn add_citizenship_to_key(
        &mut self,
        key: &str,
        region: Option<&'static str>,
        country: &str,
        count: u64,
    ) {
        match region {
            Some(slug) => {
                *self
                    .citizenships
                    .entry(key.to_string())
                    .or_default()
                    .entry(slug)
                    .or_insert(0) += count;
            }
            None => {
                *self
                    .unmapped_countries
                    .entry(country.to_string())
                    .or_insert(0) += count
            }
        }
    }

    // The plurality subregion wins; its share is of holders with a mapped country.
    pub fn assign_regions(&mut self) -> RegionStats {
        let mut stats = RegionStats::default();
        for (key, by_region) in &self.citizenships {
            let Some(entry) = self.names.get_mut(key) else {
                continue;
            };
            let total: u64 = by_region.values().sum();
            let Some((slug, top)) = by_region
                .iter()
                .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
            else {
                continue;
            };
            let sub = regions::subregion(slug).expect("table slugs are subregions");
            entry.region = Some(slug.to_string());
            entry.continent = Some(sub.continent.to_string());
            entry.region_share = Some((*top as f64 / total as f64 * 100.0).round() / 100.0);
            stats
                .regions
                .insert(slug.to_string(), sub.label.to_string());
            stats.names_with_region += 1;
        }
        stats
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
    fn merged_items_show_the_label_with_the_most_holders() {
        for order in [["Óscar", "Oscar"], ["Oscar", "Óscar"]] {
            let mut g = Graph::new();
            for label in order {
                let count = if label == "Oscar" { 40000 } else { 3000 };
                g.add_given_name(&gn(&format!("Q{label}"), label, &["Q1860"], count));
            }
            assert_eq!(g.names["oscar"].display, "Oscar", "{order:?}");
            assert_eq!(g.names["oscar"].count, 43000);
        }
    }

    #[test]
    fn merge_ties_go_to_fewer_accents_then_alphabetical() {
        let mut g = Graph::new();
        g.add_given_name(&gn("Q1", "Róbert", &["Q9058"], 100));
        g.add_given_name(&gn("Q2", "Robert", &["Q1860"], 100));
        assert_eq!(g.names["robert"].display, "Robert");
        assert_eq!(g.names["robert"].langs, ["Q1860", "Q9058"]);
        let mut g = Graph::new();
        g.add_given_name(&gn("Q3", "Hanna", &[], 50));
        g.add_given_name(&gn("Q4", "Hanná", &[], 50));
        g.add_given_name(&gn("Q5", "HANNA", &[], 50));
        assert_eq!(g.names["hanna"].display, "HANNA");
        let mut g = Graph::new();
        g.add_given_name(&gn("Q6", "Péter", &[], 90));
        g.add_given_name(&gn("Q7", "Peter", &[], 80));
        assert_eq!(g.names["peter"].display, "Péter");
    }

    #[test]
    fn multi_word_labels_never_enter_the_graph_by_any_door() {
        let mut g = Graph::new();
        g.add_given_name(&gn("Q1", "máximo merilio", &["Q1321"], 3));
        g.add_given_name(&gn("Q2", "Mary Ann", &["Q1860"], 900));
        g.add_given_name(&gn("Q3", "Máximo", &["Q1321"], 3000));
        assert_eq!(g.rejected_labels, 2);
        g.ensure_form("máximo merilio");
        g.ensure_form("S.");
        let keys: Vec<&String> = g.names.keys().collect();
        assert_eq!(keys, ["maximo"]);
        assert!(!g.key_of_qid.contains_key("Q1"));
    }

    #[test]
    fn region_is_the_plurality_subregion_with_its_share() {
        let mut g = Graph::new();
        g.add_given_name(&gn("Q1", "Bill", &["Q1860"], 1200));
        g.add_given_name(&gn("Q2", "Pierre", &["Q150"], 800));
        g.add_given_name(&gn("Q3", "Nowhere", &[], 5));
        g.add_citizenship("Q1", "Q30", 620);
        g.add_citizenship("Q1", "Q145", 300);
        g.add_citizenship("Q1", "Q16", 50);
        g.add_citizenship("Q1", "Q99999999", 230);
        g.add_citizenship("Q2", "Q142", 400);
        g.add_citizenship("Q2", "Q16", 400);
        g.add_citizenship("Q3", "Q99999999", 5);
        g.add_citizenship("Q404", "Q30", 9);
        let stats = g.assign_regions();
        assert_eq!(stats.names_with_region, 2);
        assert_eq!(
            stats.regions,
            BTreeMap::from([(
                "northern-america".to_string(),
                "Northern America".to_string()
            ),])
        );
        let bill = &g.names["bill"];
        assert_eq!(bill.region.as_deref(), Some("northern-america"));
        assert_eq!(bill.continent.as_deref(), Some("americas"));
        assert_eq!(bill.region_share, Some(0.69));
        let pierre = &g.names["pierre"];
        assert_eq!(pierre.region.as_deref(), Some("northern-america"));
        assert_eq!(pierre.region_share, Some(0.5));
        assert_eq!(g.names["nowhere"].region, None);
        assert_eq!(g.names["nowhere"].region_share, None);
        assert_eq!(
            g.unmapped_countries,
            BTreeMap::from([("Q99999999".to_string(), 235)])
        );
    }

    #[test]
    fn a_pool_only_name_takes_its_people_s_citizenship_labels() {
        let mut g = Graph::new();
        g.ensure_form("Hifikepunye");
        g.add_citizenship_label("hifikepunye", "Namibia");
        g.add_citizenship_label("hifikepunye", "Namibia");
        g.add_citizenship_label("hifikepunye", "Angola");
        g.add_citizenship_label("hifikepunye", "Wakanda");
        g.add_citizenship_label("nobody", "Namibia");
        let stats = g.assign_regions();
        assert_eq!(stats.names_with_region, 1);
        let n = &g.names["hifikepunye"];
        assert_eq!(n.region.as_deref(), Some("southern-africa"));
        assert_eq!(n.continent.as_deref(), Some("africa"));
        assert_eq!(n.region_share, Some(0.67));
        assert_eq!(
            g.unmapped_countries,
            BTreeMap::from([("Wakanda".to_string(), 1)])
        );
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

    #[test]
    fn lowercase_labels_show_capitalised() {
        assert_eq!(cased("anal"), "Anal");
        assert_eq!(cased("makarona"), "Makarona");
        assert_eq!(cased("HANNA"), "HANNA");
        assert_eq!(cased("Óscar"), "Óscar");
        assert_eq!(cased("jean-paul"), "Jean-Paul");
    }
}
