use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skip {
    NoLabel,
    NoGivenName,
    Licence,
    MissingFile,
    FetchError,
    Undecodable,
    NoFace,
    ManyFaces,
    SmallFace,
}

impl Skip {
    pub const fn as_str(self) -> &'static str {
        match self {
            Skip::NoLabel => "no-label",
            Skip::NoGivenName => "no-given-name",
            Skip::Licence => "licence",
            Skip::MissingFile => "missing-file",
            Skip::FetchError => "fetch-error",
            Skip::Undecodable => "undecodable",
            Skip::NoFace => "no-face",
            Skip::ManyFaces => "many-faces",
            Skip::SmallFace => "small-face",
        }
    }
}

impl std::fmt::Display for Skip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ImageInfo {
    pub file: String,
    pub artist: Option<String>,
    pub licence: String,
    #[serde(rename = "licenceUrl")]
    pub licence_url: Option<String>,
    #[serde(rename = "pageUrl")]
    pub page_url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Person {
    pub qid: String,
    pub label: String,
    pub display: String,
    pub names: Vec<String>,
    #[serde(
        rename = "formDisplays",
        default,
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub form_displays: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nicknames: Vec<String>,
    pub born: i32,
    pub citizenship: Vec<String>,
    pub occupations: Vec<String>,
    pub description: Option<String>,
    pub wiki: Option<String>,
    pub crop: String,
    pub image: ImageInfo,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub retired: bool,
}

impl Person {
    #[cfg(test)]
    pub fn stub(qid: &str) -> Person {
        Person {
            qid: qid.to_string(),
            label: String::new(),
            display: String::new(),
            names: vec![],
            form_displays: BTreeMap::new(),
            nicknames: vec![],
            born: 0,
            citizenship: vec![],
            occupations: vec![],
            description: None,
            wiki: None,
            crop: String::new(),
            image: ImageInfo {
                file: String::new(),
                artist: None,
                licence: String::new(),
                licence_url: None,
                page_url: String::new(),
            },
            retired: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Pool {
    pub version: u32,
    pub generated: String,
    pub people: Vec<Person>,
}

impl Pool {
    pub fn empty(today: &str) -> Self {
        Pool {
            version: 1,
            generated: today.to_string(),
            people: Vec::new(),
        }
    }

    pub fn upsert(&mut self, person: Person) {
        match self.people.iter_mut().find(|p| p.qid == person.qid) {
            Some(slot) => *slot = person,
            None => self.people.push(person),
        }
    }

    pub fn sort(&mut self) {
        self.people.sort_by_key(|p| qid_order(&p.qid));
    }

    pub fn index(&self) -> HashMap<String, usize> {
        self.people
            .iter()
            .enumerate()
            .map(|(i, p)| (p.qid.clone(), i))
            .collect()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Name {
    pub display: String,
    pub langs: Vec<String>,
    pub families: Vec<String>,
    pub count: u64,
    pub dm: String,
    pub rhyme: String,
    pub era: Option<i32>,
    #[serde(rename = "sameAs")]
    pub same_as: Vec<String>,
    #[serde(rename = "shortOf")]
    pub short_of: Vec<String>,
    pub region: Option<String>,
    pub continent: Option<String>,
    #[serde(rename = "regionShare")]
    pub region_share: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Names {
    pub version: u32,
    pub languages: BTreeMap<String, String>,
    pub regions: BTreeMap<String, String>,
    pub names: BTreeMap<String, Name>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub version: u32,
    #[serde(rename = "lastRun")]
    pub last_run: String,
    pub processed: BTreeSet<String>,
    pub skipped: BTreeMap<String, String>,
}

impl State {
    pub fn empty(today: &str) -> Self {
        State {
            version: 1,
            last_run: today.to_string(),
            processed: BTreeSet::new(),
            skipped: BTreeMap::new(),
        }
    }

    pub fn needs_fetch(&self, qid: &str) -> bool {
        if self.processed.contains(qid) {
            return false;
        }
        match self.skipped.get(qid) {
            None => true,
            Some(reason) => reason == Skip::FetchError.as_str(),
        }
    }

    // state.json is written more often than pool.json, so a run killed between
    // the two can list people the pool never received; they are fetched again.
    pub fn reconcile(&mut self, pool: &Pool) -> usize {
        let index = pool.index();
        let before = self.processed.len();
        self.processed.retain(|q| index.contains_key(q));
        before - self.processed.len()
    }

    pub fn mark_processed(&mut self, qid: &str) {
        self.skipped.remove(qid);
        self.processed.insert(qid.to_string());
    }

    pub fn mark_skipped(&mut self, qid: &str, reason: Skip) {
        self.skipped
            .insert(qid.to_string(), reason.as_str().to_string());
    }
}

pub fn qid_order(qid: &str) -> (u64, String) {
    let n = qid
        .strip_prefix('Q')
        .and_then(|s| s.parse().ok())
        .unwrap_or(u64::MAX);
    (n, qid.to_string())
}

pub fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let v = serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(Some(v))
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).with_context(|| format!("write {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("rename to {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processed_qid_is_not_fetched_again() {
        let mut s = State::empty("2026-01-01");
        s.mark_processed("Q42");
        assert!(!s.needs_fetch("Q42"));
        assert!(s.needs_fetch("Q43"));
    }

    #[test]
    fn skipped_qid_is_retried_only_after_a_transient_error() {
        let mut s = State::empty("2026-01-01");
        s.mark_skipped("Q1", Skip::Licence);
        s.mark_skipped("Q2", Skip::NoFace);
        s.mark_skipped("Q3", Skip::FetchError);
        assert!(!s.needs_fetch("Q1"));
        assert!(!s.needs_fetch("Q2"));
        assert!(s.needs_fetch("Q3"));
        s.mark_processed("Q3");
        assert!(!s.skipped.contains_key("Q3"));
        assert!(s.processed.contains("Q3"));
    }

    #[test]
    fn state_round_trips_through_json_in_sorted_order() {
        let mut s = State::empty("2026-01-01");
        s.mark_processed("Q9");
        s.mark_processed("Q10");
        s.mark_skipped("Q5", Skip::Licence);
        let text = serde_json::to_string(&s).unwrap();
        assert!(text.contains(r#""processed":["Q10","Q9"]"#));
        assert!(text.contains(r#""skipped":{"Q5":"licence"}"#));
        let back: State = serde_json::from_str(&text).unwrap();
        assert_eq!(back.processed.len(), 2);
    }

    #[test]
    fn name_record_has_the_contract_fields_in_order() {
        let n = Name {
            display: "Bill".into(),
            langs: vec!["Q1860".into()],
            families: vec!["germanic".into()],
            count: 1200,
            dm: "PL".into(),
            rhyme: "PL".into(),
            era: None,
            same_as: vec![],
            short_of: vec!["william".into()],
            region: Some("northern-america".into()),
            continent: Some("americas".into()),
            region_share: Some(0.62),
        };
        assert_eq!(
            serde_json::to_string(&n).unwrap(),
            r#"{"display":"Bill","langs":["Q1860"],"families":["germanic"],"count":1200,"dm":"PL","rhyme":"PL","era":null,"sameAs":[],"shortOf":["william"],"region":"northern-america","continent":"americas","regionShare":0.62}"#
        );
        let mut nowhere = n.clone();
        nowhere.region = None;
        nowhere.continent = None;
        nowhere.region_share = None;
        assert!(serde_json::to_string(&nowhere)
            .unwrap()
            .ends_with(r#""region":null,"continent":null,"regionShare":null}"#));
        let mut with_era = n.clone();
        with_era.era = Some(1952);
        assert!(serde_json::to_string(&with_era)
            .unwrap()
            .contains(r#""era":1952"#));
    }

    #[test]
    fn retired_flag_is_omitted_when_false() {
        let p = Person {
            qid: "Q1".into(),
            label: "A B".into(),
            display: "A".into(),
            names: vec!["a".into()],
            form_displays: BTreeMap::new(),
            nicknames: vec![],
            born: 1950,
            citizenship: vec![],
            occupations: vec![],
            description: None,
            wiki: None,
            crop: "crops/Q1.jpg".into(),
            image: ImageInfo {
                file: "A.jpg".into(),
                artist: None,
                licence: "CC0".into(),
                licence_url: None,
                page_url: "https://commons.wikimedia.org/wiki/File:A.jpg".into(),
            },
            retired: false,
        };
        let text = serde_json::to_string(&p).unwrap();
        assert!(!text.contains("retired"));
        assert!(!text.contains("formDisplays"));
        assert!(!text.contains("nicknames"));
        let mut cased = p.clone();
        cased.form_displays.insert("a".into(), "Á".into());
        assert!(serde_json::to_string(&cased)
            .unwrap()
            .contains(r#""formDisplays":{"a":"Á"}"#));
        let back: Person = serde_json::from_str(&text).unwrap();
        assert!(back.form_displays.is_empty());
        let mut r = p.clone();
        r.retired = true;
        assert!(serde_json::to_string(&r)
            .unwrap()
            .contains(r#""retired":true"#));
    }

    fn pool_of(qids: &[&str]) -> Pool {
        let mut pool = Pool::empty("2026-01-01");
        pool.people.extend(qids.iter().map(|q| Person::stub(q)));
        pool
    }

    #[test]
    fn pool_sorts_by_numeric_qid() {
        let mut pool = pool_of(&["Q100", "Q9", "Q42"]);
        pool.sort();
        let order: Vec<&str> = pool.people.iter().map(|p| p.qid.as_str()).collect();
        assert_eq!(order, ["Q9", "Q42", "Q100"]);
        let index = pool.index();
        assert_eq!(index["Q42"], 1);
        assert_eq!(index.len(), 3);
    }

    #[test]
    fn processed_people_missing_from_the_pool_are_fetched_again() {
        let pool = pool_of(&["Q9", "Q42"]);
        let mut s = State::empty("2026-01-01");
        s.mark_processed("Q9");
        s.mark_processed("Q42");
        s.mark_processed("Q100");
        s.mark_skipped("Q7", Skip::NoFace);
        assert_eq!(s.reconcile(&pool), 1);
        assert!(!s.needs_fetch("Q9"));
        assert!(s.needs_fetch("Q100"));
        assert!(!s.needs_fetch("Q7"));
        assert_eq!(s.reconcile(&pool), 0);
    }
}
