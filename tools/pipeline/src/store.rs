use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const TRANSIENT_SKIP: &str = "fetch-error";

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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Names {
    pub version: u32,
    pub languages: BTreeMap<String, String>,
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
            Some(reason) => reason == TRANSIENT_SKIP,
        }
    }

    pub fn mark_processed(&mut self, qid: &str) {
        self.skipped.remove(qid);
        self.processed.insert(qid.to_string());
    }

    pub fn mark_skipped(&mut self, qid: &str, reason: &str) {
        self.skipped.insert(qid.to_string(), reason.to_string());
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
        s.mark_skipped("Q1", "licence");
        s.mark_skipped("Q2", "no-face");
        s.mark_skipped("Q3", TRANSIENT_SKIP);
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
        s.mark_skipped("Q5", "licence");
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
        };
        assert_eq!(
            serde_json::to_string(&n).unwrap(),
            r#"{"display":"Bill","langs":["Q1860"],"families":["germanic"],"count":1200,"dm":"PL","rhyme":"PL","era":null,"sameAs":[],"shortOf":["william"]}"#
        );
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
        let mut r = p.clone();
        r.retired = true;
        assert!(serde_json::to_string(&r)
            .unwrap()
            .contains(r#""retired":true"#));
    }

    #[test]
    fn pool_sorts_by_numeric_qid() {
        let mut pool = Pool::empty("2026-01-01");
        for q in ["Q100", "Q9", "Q42"] {
            pool.people.push(Person {
                qid: q.into(),
                label: String::new(),
                display: String::new(),
                names: vec![],
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
            });
        }
        pool.sort();
        let order: Vec<&str> = pool.people.iter().map(|p| p.qid.as_str()).collect();
        assert_eq!(order, ["Q9", "Q42", "Q100"]);
    }
}
