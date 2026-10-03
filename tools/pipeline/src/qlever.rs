use crate::http::Client;
use crate::names::GivenName;
use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;

pub const ENDPOINT: &str = "https://qlever.dev/api/wikidata";
pub const PAGE: usize = 2000;
const VALUES_BATCH: usize = 500;
const YEARS_BATCH: usize = 400;

const PREFIXES: &str = "\
PREFIX wd: <http://www.wikidata.org/entity/>
PREFIX wdt: <http://www.wikidata.org/prop/direct/>
PREFIX wikibase: <http://wikiba.se/ontology#>
PREFIX schema: <http://schema.org/>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
";

pub type Row = BTreeMap<String, String>;

#[derive(Debug, Clone)]
pub struct Candidate {
    pub qid: String,
    pub label: String,
    pub description: Option<String>,
    pub file: String,
    pub born: i32,
    pub wiki: Option<String>,
    pub givens: Vec<(String, String)>,
    pub nicknames: Vec<String>,
    pub citizenship: Vec<String>,
    pub occupations: Vec<String>,
}

pub fn query(client: &mut Client, endpoint: &str, sparql: &str) -> Result<Vec<Row>> {
    let full = format!("{PREFIXES}{sparql}");
    let resp = client
        .send(endpoint, |c, u| {
            c.post(u)
                .form(&[("query", full.as_str())])
                .header("Accept", "application/sparql-results+json")
        })
        .map_err(|e| anyhow!("qlever: {e}"))?;
    let body: Value = resp.json().context("qlever json")?;
    let bindings = body
        .pointer("/results/bindings")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("qlever response without bindings: {}", body))?;
    Ok(bindings
        .iter()
        .map(|b| {
            b.as_object()
                .map(|o| {
                    o.iter()
                        .filter_map(|(k, v)| {
                            v.get("value")
                                .and_then(Value::as_str)
                                .map(|s| (k.clone(), s.to_string()))
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect())
}

pub fn paged<F>(client: &mut Client, endpoint: &str, body: &str, mut on_page: F) -> Result<()>
where
    F: FnMut(Vec<Row>) -> Result<bool>,
{
    let mut offset = 0;
    loop {
        let sparql = format!("{body}\nLIMIT {PAGE} OFFSET {offset}");
        let rows = query(client, endpoint, &sparql)?;
        let n = rows.len();
        let keep_going = on_page(rows)?;
        if n < PAGE || !keep_going {
            return Ok(());
        }
        offset += PAGE;
    }
}

pub fn qid_of(uri: &str) -> &str {
    uri.rsplit('/').next().unwrap_or(uri)
}

pub fn pool_query() -> String {
    r#"SELECT ?p ?label ?desc ?img ?dob ?article
  (GROUP_CONCAT(DISTINCT ?gnpair; SEPARATOR="|") AS ?givens)
  (GROUP_CONCAT(DISTINCT ?nick; SEPARATOR="|") AS ?nicks)
  (GROUP_CONCAT(DISTINCT ?cLabel; SEPARATOR="|") AS ?citizen)
  (GROUP_CONCAT(DISTINCT ?oLabel; SEPARATOR="|") AS ?occ)
WHERE {
  {
    SELECT ?p (SAMPLE(?i) AS ?img) (MIN(?d) AS ?dob) WHERE {
      ?p wdt:P31 wd:Q5 ; wdt:P18 ?i ; wdt:P569 ?d ; wikibase:sitelinks ?sl .
      FILTER(xsd:integer(?sl) >= 40)
      FILTER(?d >= "1900-01-01T00:00:00Z"^^xsd:dateTime)
    }
    GROUP BY ?p
  }
  OPTIONAL { ?p rdfs:label ?labelEn FILTER(LANG(?labelEn) = "en") }
  OPTIONAL { ?p rdfs:label ?labelMul FILTER(LANG(?labelMul) = "mul") }
  BIND(COALESCE(?labelEn, ?labelMul) AS ?label)
  OPTIONAL { ?p schema:description ?desc FILTER(LANG(?desc) = "en") }
  OPTIONAL { ?article schema:about ?p ; schema:isPartOf <https://en.wikipedia.org/> }
  OPTIONAL {
    ?p wdt:P735 ?gn .
    OPTIONAL { ?gn rdfs:label ?gnEn FILTER(LANG(?gnEn) = "en") }
    OPTIONAL { ?gn rdfs:label ?gnMul FILTER(LANG(?gnMul) = "mul") }
    BIND(CONCAT(STRAFTER(STR(?gn), "entity/"), "=", COALESCE(?gnEn, ?gnMul, "")) AS ?gnpair)
  }
  OPTIONAL { ?p wdt:P1449 ?nick }
  OPTIONAL {
    ?p wdt:P27 ?c .
    OPTIONAL { ?c rdfs:label ?cEn FILTER(LANG(?cEn) = "en") }
    OPTIONAL { ?c rdfs:label ?cMul FILTER(LANG(?cMul) = "mul") }
    BIND(COALESCE(?cEn, ?cMul) AS ?cLabel)
  }
  OPTIONAL {
    ?p wdt:P106 ?o .
    OPTIONAL { ?o rdfs:label ?oEn FILTER(LANG(?oEn) = "en") }
    OPTIONAL { ?o rdfs:label ?oMul FILTER(LANG(?oMul) = "mul") }
    BIND(COALESCE(?oEn, ?oMul) AS ?oLabel)
  }
}
GROUP BY ?p ?label ?desc ?img ?dob ?article
ORDER BY ?p"#
        .to_string()
}

const GIVEN_NAME_LANGS: &str = r#"(GROUP_CONCAT(DISTINCT STRAFTER(STR(?lang), "entity/"); SEPARATOR="|") AS ?langs)
  (GROUP_CONCAT(DISTINCT STRAFTER(STR(?same), "entity/"); SEPARATOR="|") AS ?sameAs)
  (MAX(?hyp) AS ?hypocorism)"#;

const GIVEN_NAME_LINKS: &str = r#"OPTIONAL { { ?gn wdt:P460 ?same } UNION { ?same wdt:P460 ?gn } }
  OPTIONAL { { ?gn wdt:P31 wd:Q1130279 } UNION { ?gn wdt:P366 wd:Q1130279 } BIND(1 AS ?hyp) }"#;

pub fn common_given_names_query() -> String {
    format!(
        r#"SELECT ?gn ?label ?n {GIVEN_NAME_LANGS} WHERE {{
  {{ SELECT ?gn (COUNT(?h) AS ?n) WHERE {{ ?h wdt:P735 ?gn }} GROUP BY ?gn HAVING (?n >= 50) }}
  OPTIONAL {{ ?gn rdfs:label ?labelEn FILTER(LANG(?labelEn) = "en") }}
  OPTIONAL {{ ?gn rdfs:label ?labelMul FILTER(LANG(?labelMul) = "mul") }}
  BIND(COALESCE(?labelEn, ?labelMul) AS ?label)
  OPTIONAL {{ ?gn wdt:P407 ?lang }}
  {GIVEN_NAME_LINKS}
}}
GROUP BY ?gn ?label ?n
ORDER BY ?gn"#
    )
}

fn given_names_by_id_query(qids: &[&str]) -> String {
    let values: Vec<String> = qids.iter().map(|q| format!("wd:{q}")).collect();
    format!(
        r#"SELECT ?gn ?label (COUNT(DISTINCT ?h) AS ?n) {GIVEN_NAME_LANGS} WHERE {{
  VALUES ?gn {{ {} }}
  OPTIONAL {{ ?h wdt:P735 ?gn }}
  OPTIONAL {{ ?gn rdfs:label ?labelEn FILTER(LANG(?labelEn) = "en") }}
  OPTIONAL {{ ?gn rdfs:label ?labelMul FILTER(LANG(?labelMul) = "mul") }}
  BIND(COALESCE(?labelEn, ?labelMul) AS ?label)
  OPTIONAL {{ ?gn wdt:P407 ?lang }}
  {GIVEN_NAME_LINKS}
}}
GROUP BY ?gn ?label"#,
        values.join(" ")
    )
}

fn birth_years_query(qids: &[&str]) -> String {
    let values: Vec<String> = qids.iter().map(|q| format!("wd:{q}")).collect();
    format!(
        r#"SELECT ?gn ?y (COUNT(?h) AS ?n) WHERE {{
  VALUES ?gn {{ {} }}
  ?h wdt:P735 ?gn ; wdt:P569 ?dob .
  BIND(YEAR(?dob) AS ?y)
}}
GROUP BY ?gn ?y"#,
        values.join(" ")
    )
}

fn labels_query(qids: &[&str]) -> String {
    let values: Vec<String> = qids.iter().map(|q| format!("wd:{q}")).collect();
    format!(
        r#"SELECT ?item ?label WHERE {{
  VALUES ?item {{ {} }}
  OPTIONAL {{ ?item rdfs:label ?labelEn FILTER(LANG(?labelEn) = "en") }}
  OPTIONAL {{ ?item rdfs:label ?labelMul FILTER(LANG(?labelMul) = "mul") }}
  BIND(COALESCE(?labelEn, ?labelMul) AS ?label)
}}"#,
        values.join(" ")
    )
}

fn split_multi(s: Option<&String>) -> Vec<String> {
    let mut v: Vec<String> = s
        .map(|s| {
            s.split('|')
                .filter(|x| !x.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v.dedup();
    v
}

pub fn candidate_from_row(row: &Row) -> Option<Candidate> {
    let qid = qid_of(row.get("p")?).to_string();
    let file = file_name_from_filepath(row.get("img")?)?;
    let born: i32 = row.get("dob")?.get(..4)?.parse().ok()?;
    let mut givens: Vec<(String, String)> = split_multi(row.get("givens"))
        .into_iter()
        .filter_map(|pair| {
            let (q, label) = pair.split_once('=')?;
            if q.is_empty() {
                return None;
            }
            Some((q.to_string(), label.to_string()))
        })
        .collect();
    givens.sort_by_key(|(q, _)| crate::store::qid_order(q));
    let wiki = row
        .get("article")
        .and_then(|a| a.strip_prefix("https://en.wikipedia.org/wiki/"))
        .map(percent_decode)
        .map(|t| t.replace('_', " "));
    let mut occupations = split_multi(row.get("occ"));
    occupations.truncate(3);
    Some(Candidate {
        qid,
        label: row.get("label").cloned().unwrap_or_default(),
        description: row.get("desc").cloned().filter(|s| !s.is_empty()),
        file,
        born,
        wiki,
        givens,
        nicknames: split_multi(row.get("nicks")),
        citizenship: split_multi(row.get("citizen")),
        occupations,
    })
}

pub fn file_name_from_filepath(uri: &str) -> Option<String> {
    let rest = uri.split("Special:FilePath/").nth(1)?;
    let name = percent_decode(rest).replace('_', " ");
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

pub fn percent_decode(s: &str) -> String {
    percent_encoding::percent_decode_str(s)
        .decode_utf8_lossy()
        .into_owned()
}

fn given_name_from_row(row: &Row) -> Option<GivenName> {
    Some(GivenName {
        qid: qid_of(row.get("gn")?).to_string(),
        label: row.get("label").cloned().unwrap_or_default(),
        langs: split_multi(row.get("langs"))
            .iter()
            .map(|u| qid_of(u).to_string())
            .collect(),
        count: row.get("n").and_then(|n| n.parse().ok()).unwrap_or(0),
        same_as: split_multi(row.get("sameAs"))
            .iter()
            .map(|u| qid_of(u).to_string())
            .collect(),
        hypocorism: row.get("hypocorism").is_some_and(|h| h == "1"),
    })
}

pub struct BirthYearRow {
    pub qid: String,
    pub year: i32,
    pub count: u64,
}

pub fn birth_years(client: &mut Client, qids: &[String]) -> Result<Vec<BirthYearRow>> {
    let mut out = Vec::new();
    for chunk in qids.chunks(YEARS_BATCH) {
        let refs: Vec<&str> = chunk.iter().map(String::as_str).collect();
        for row in query(client, ENDPOINT, &birth_years_query(&refs))? {
            let (Some(gn), Some(y), Some(n)) = (row.get("gn"), row.get("y"), row.get("n")) else {
                continue;
            };
            let (Ok(year), Ok(count)) = (y.parse(), n.parse()) else {
                continue;
            };
            out.push(BirthYearRow {
                qid: qid_of(gn).to_string(),
                year,
                count,
            });
        }
    }
    Ok(out)
}

pub fn common_given_names(client: &mut Client) -> Result<Vec<GivenName>> {
    let mut out = Vec::new();
    paged(client, ENDPOINT, &common_given_names_query(), |rows| {
        out.extend(rows.iter().filter_map(given_name_from_row));
        Ok(true)
    })?;
    Ok(out)
}

pub fn given_names_by_id(client: &mut Client, qids: &[String]) -> Result<Vec<GivenName>> {
    let mut out = Vec::new();
    for chunk in qids.chunks(VALUES_BATCH) {
        let refs: Vec<&str> = chunk.iter().map(String::as_str).collect();
        let rows = query(client, ENDPOINT, &given_names_by_id_query(&refs))?;
        out.extend(rows.iter().filter_map(given_name_from_row));
    }
    Ok(out)
}

pub fn labels(client: &mut Client, qids: &[String]) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for chunk in qids.chunks(VALUES_BATCH) {
        let refs: Vec<&str> = chunk.iter().map(String::as_str).collect();
        for row in query(client, ENDPOINT, &labels_query(&refs))? {
            if let (Some(item), Some(label)) = (row.get("item"), row.get("label")) {
                out.insert(qid_of(item).to_string(), label.clone());
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pairs: &[(&str, &str)]) -> Row {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn file_name_is_decoded_from_filepath_uri() {
        assert_eq!(
            file_name_from_filepath("http://commons.wikimedia.org/wiki/Special:FilePath/Anton%C3%ADn%20Novotn%C3%BD%201968.jpg").as_deref(),
            Some("Antonín Novotný 1968.jpg")
        );
        assert_eq!(file_name_from_filepath("http://example.org/x"), None);
    }

    #[test]
    fn candidate_parses_sample_shape() {
        let r = row(&[
            ("p", "http://www.wikidata.org/entity/Q238739"),
            ("label", "George Wells Beadle"),
            ("desc", "American geneticist"),
            (
                "img",
                "http://commons.wikimedia.org/wiki/Special:FilePath/George%20Beadle.jpg",
            ),
            ("dob", "1903-10-22T00:00:00Z"),
            (
                "article",
                "https://en.wikipedia.org/wiki/George_Wells_Beadle",
            ),
            ("givens", "Q15921732=Wells|Q15921732=Wells|Q1370783=George"),
            ("nicks", ""),
            ("citizen", "United States of America"),
            ("occ", "geneticist|university teacher|biologist|writer"),
        ]);
        let c = candidate_from_row(&r).unwrap();
        assert_eq!(c.qid, "Q238739");
        assert_eq!(c.born, 1903);
        assert_eq!(c.file, "George Beadle.jpg");
        assert_eq!(c.wiki.as_deref(), Some("George Wells Beadle"));
        assert_eq!(
            c.givens,
            vec![
                ("Q1370783".to_string(), "George".to_string()),
                ("Q15921732".to_string(), "Wells".to_string())
            ]
        );
        assert!(c.nicknames.is_empty());
        assert_eq!(
            c.occupations,
            vec!["biologist", "geneticist", "university teacher"]
        );
    }

    #[test]
    fn given_name_row_carries_links_and_the_hypocorism_flag() {
        let r = row(&[
            ("gn", "http://www.wikidata.org/entity/Q18245781"),
            ("label", "Bill"),
            ("n", "1200"),
            ("langs", "Q1860"),
            ("sameAs", "Q12344159|Q15282375"),
            ("hypocorism", "1"),
        ]);
        let g = given_name_from_row(&r).unwrap();
        assert_eq!(g.same_as, vec!["Q12344159", "Q15282375"]);
        assert!(g.hypocorism);
        let r = row(&[("gn", "http://www.wikidata.org/entity/Q1"), ("n", "60")]);
        let g = given_name_from_row(&r).unwrap();
        assert!(g.same_as.is_empty());
        assert!(!g.hypocorism);
    }

    #[test]
    fn given_name_without_label_keeps_qid() {
        let r = row(&[
            ("p", "http://www.wikidata.org/entity/Q1"),
            (
                "img",
                "http://commons.wikimedia.org/wiki/Special:FilePath/a.jpg",
            ),
            ("dob", "1950-01-01T00:00:00Z"),
            ("givens", "Q5="),
        ]);
        let c = candidate_from_row(&r).unwrap();
        assert_eq!(c.givens, vec![("Q5".to_string(), String::new())]);
    }
}
