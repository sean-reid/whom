use crate::commons::{self, FileStatus};
use crate::face::{self, FaceDetector, Outcome};
use crate::http::{Client, FetchError};
use crate::names::{GivenName, Graph};
use crate::qlever::{self, Candidate};
use crate::store::{self, ImageInfo, Names, Person, Pool, State, TRANSIENT_SKIP};
use crate::text::{first_token, normalize};
use anyhow::{bail, Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const PENDING_UPLOADS: &str = "pending-uploads.txt";
const SAVE_EVERY: usize = 25;

pub struct Args {
    pub out: PathBuf,
    pub max_fetch: usize,
    pub sample: Option<usize>,
}

#[derive(Default)]
struct Report {
    scanned: usize,
    to_fetch: usize,
    fetched: usize,
    skipped: BTreeMap<String, usize>,
}

impl Report {
    fn skip(&mut self, reason: &str) {
        *self.skipped.entry(reason.to_string()).or_insert(0) += 1;
    }
}

pub fn today() -> String {
    time::OffsetDateTime::now_utc().date().to_string()
}

pub fn run(args: &Args) -> Result<()> {
    let started = Instant::now();
    let today = today();
    let crops_dir = args.out.join("crops");
    std::fs::create_dir_all(&crops_dir).context("create crops dir")?;

    let mut state: State =
        store::read_json(&args.out.join("state.json"))?.unwrap_or_else(|| State::empty(&today));
    let mut pool: Pool =
        store::read_json(&args.out.join("pool.json"))?.unwrap_or_else(|| Pool::empty(&today));
    let mut client = Client::new()?;
    let mut report = Report::default();

    let candidates = scan(&mut client, &state, args.sample)?;
    report.scanned = candidates.len();
    println!("scanned {} people", candidates.len());

    let seen: BTreeSet<&str> = candidates.iter().map(|c| c.qid.as_str()).collect();
    if args.sample.is_none() {
        for p in &mut pool.people {
            p.retired = !seen.contains(p.qid.as_str());
        }
    }
    for c in &candidates {
        if let Some(p) = pool.people.iter_mut().find(|p| p.qid == c.qid) {
            refresh(p, c);
        }
    }

    let mut todo: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| state.needs_fetch(&c.qid))
        .collect();
    todo.sort_by_key(|c| store::qid_order(&c.qid));
    if let Some(n) = args.sample {
        todo.truncate(n);
    }
    report.to_fetch = todo.len();
    println!("{} people to fetch", todo.len());

    let mut with_names = Vec::new();
    for c in todo {
        if c.label.trim().is_empty() {
            state.mark_skipped(&c.qid, "no-label");
            report.skip("no-label");
        } else if c.givens.is_empty() {
            state.mark_skipped(&c.qid, "no-given-name");
            report.skip("no-given-name");
        } else {
            with_names.push(c);
        }
    }

    let files: Vec<String> = with_names.iter().map(|c| c.file.clone()).collect();
    let licences = commons::licences(&mut client, &files)?;
    let mut pending = Vec::new();
    let mut detector = FaceDetector::new()?;
    let mut fetched = 0;
    let outcome = (|| -> Result<()> {
        for c in &with_names {
            let licence = match licences.get(&c.file) {
                Some(FileStatus::Licensed(l)) => l.clone(),
                Some(FileStatus::Rejected(reason)) => {
                    state.mark_skipped(&c.qid, reason);
                    report.skip(reason);
                    continue;
                }
                Some(FileStatus::Missing) | None => {
                    state.mark_skipped(&c.qid, "missing-file");
                    report.skip("missing-file");
                    continue;
                }
            };
            if fetched >= args.max_fetch {
                break;
            }
            fetched += 1;
            let reason = match fetch_and_crop(&mut client, &mut detector, c, &crops_dir) {
                Ok(()) => None,
                Err(Step::Skip(reason)) => Some(reason),
                Err(Step::Fatal(e)) => return Err(e),
            };
            match reason {
                Some(reason) => {
                    state.mark_skipped(&c.qid, &reason);
                    report.skip(&reason);
                }
                None => {
                    state.mark_processed(&c.qid);
                    report.fetched += 1;
                    pending.push(format!("crops/{}.jpg", c.qid));
                    pool.upsert(person_from(c, &licence));
                    println!("  {} {} ok", c.qid, c.label);
                }
            }
            if (report.fetched + report.skipped.values().sum::<usize>()) % SAVE_EVERY == 0 {
                save(&args.out, &mut pool, &state, &today)?;
            }
        }
        Ok(())
    })();
    save(&args.out, &mut pool, &state, &today)?;
    append_pending(&args.out, &pending)?;
    if let Err(e) = outcome {
        bail!("run stopped: {e}");
    }

    let graph = build_graph(&mut client, &candidates, &pool)?;
    let languages = qlever::labels(
        &mut client,
        &graph.langs_seen.iter().cloned().collect::<Vec<_>>(),
    )?;
    let names = Names {
        version: 1,
        languages,
        names: graph.names,
    };
    store::write_json(&args.out.join("names.json"), &names)?;
    append_pending(&args.out, &["names.json".to_string()])?;

    println!();
    println!("people scanned: {}", report.scanned);
    println!("people needing a fetch: {}", report.to_fetch);
    println!("people added: {}", report.fetched);
    println!(
        "pool size: {} ({} retired)",
        pool.people.len(),
        pool.people.iter().filter(|p| p.retired).count()
    );
    println!("names in graph: {}", names.names.len());
    println!("given names without an English label: {}", graph.unlabelled);
    println!("languages seen: {}", graph.langs_seen.len());
    println!(
        "languages without a family: {} [{}]",
        graph.langs_without_family.len(),
        graph
            .langs_without_family
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("skips this run:");
    for (reason, n) in &report.skipped {
        println!("  {reason}: {n}");
    }
    println!("requests: {}", client.requests);
    println!("wall time: {:.0}s", started.elapsed().as_secs_f64());
    Ok(())
}

fn scan(client: &mut Client, state: &State, sample: Option<usize>) -> Result<Vec<Candidate>> {
    let mut candidates = Vec::new();
    qlever::paged(client, &qlever::pool_query(), |rows| {
        candidates.extend(rows.iter().filter_map(qlever::candidate_from_row));
        Ok(match sample {
            Some(n) => {
                candidates
                    .iter()
                    .filter(|c| state.needs_fetch(&c.qid))
                    .count()
                    < n
            }
            None => true,
        })
    })?;
    Ok(candidates)
}

enum Step {
    Skip(String),
    Fatal(anyhow::Error),
}

fn fetch_and_crop(
    client: &mut Client,
    detector: &mut FaceDetector,
    c: &Candidate,
    crops_dir: &Path,
) -> Result<(), Step> {
    let thumb = match commons::fetch_thumb(client, &c.file) {
        Ok(t) => t,
        Err(FetchError::NotFound) => return Err(Step::Skip("missing-file".into())),
        Err(FetchError::Transient(e)) => {
            eprintln!("  {} {e}", c.qid);
            return Err(Step::Skip(TRANSIENT_SKIP.into()));
        }
        Err(e @ FetchError::Fatal(_)) => return Err(Step::Fatal(e.into())),
    };
    let Some(img) = face::decode(&thumb.content_type, &thumb.bytes) else {
        return Err(Step::Skip("undecodable".into()));
    };
    let found = match detector.detect(&img) {
        Outcome::One(f) => f,
        Outcome::None => return Err(Step::Skip("no-face".into())),
        Outcome::Many(n) => {
            eprintln!("  {} {n} faces", c.qid);
            return Err(Step::Skip("many-faces".into()));
        }
        Outcome::Small(w) => {
            eprintln!("  {} face {w} px wide", c.qid);
            return Err(Step::Skip("small-face".into()));
        }
    };
    let crop = face::crop(&img, &found);
    let bytes = face::encode_jpeg(&crop).map_err(Step::Fatal)?;
    std::fs::write(crops_dir.join(format!("{}.jpg", c.qid)), bytes)
        .context("write crop")
        .map_err(Step::Fatal)?;
    Ok(())
}

pub fn display_name(c: &Candidate) -> String {
    let token = first_token(&c.label).unwrap_or("");
    let wanted = normalize(token);
    c.givens
        .iter()
        .find(|(_, l)| !l.is_empty() && normalize(l) == wanted)
        .or_else(|| c.givens.iter().find(|(_, l)| !l.is_empty()))
        .map(|(_, l)| l.trim().to_string())
        .unwrap_or_else(|| token.to_string())
}

pub fn name_forms(c: &Candidate) -> Vec<(String, String)> {
    let mut forms: Vec<(String, String)> = Vec::new();
    let sources = c
        .givens
        .iter()
        .map(|(_, l)| l.as_str())
        .chain(c.nicknames.iter().map(String::as_str))
        .chain(first_token(&c.label));
    for s in sources {
        let n = normalize(s);
        if !n.is_empty() && !forms.iter().any(|(k, _)| *k == n) {
            forms.push((n, s.trim().to_string()));
        }
    }
    forms.sort();
    forms
}

fn person_from(c: &Candidate, licence: &commons::Licence) -> Person {
    Person {
        qid: c.qid.clone(),
        label: c.label.clone(),
        display: display_name(c),
        names: name_forms(c).into_iter().map(|(k, _)| k).collect(),
        born: c.born,
        citizenship: c.citizenship.clone(),
        occupations: c.occupations.clone(),
        description: c.description.clone(),
        wiki: c.wiki.clone(),
        crop: format!("crops/{}.jpg", c.qid),
        image: ImageInfo {
            file: c.file.clone(),
            artist: licence.artist.clone(),
            licence: licence.short_name.clone(),
            licence_url: licence.url.clone(),
            page_url: commons::page_url(&c.file),
        },
        retired: false,
    }
}

fn refresh(p: &mut Person, c: &Candidate) {
    p.label = c.label.clone();
    p.display = display_name(c);
    p.names = name_forms(c).into_iter().map(|(k, _)| k).collect();
    p.born = c.born;
    p.citizenship = c.citizenship.clone();
    p.occupations = c.occupations.clone();
    p.description = c.description.clone();
    p.wiki = c.wiki.clone();
}

fn build_graph(client: &mut Client, candidates: &[Candidate], pool: &Pool) -> Result<Graph> {
    let mut graph = Graph::new();
    let common = qlever::common_given_names(client)?;
    println!("given names with 50 or more holders: {}", common.len());
    let known: BTreeSet<&str> = common.iter().map(|g| g.qid.as_str()).collect();
    let mut extra: Vec<String> = candidates
        .iter()
        .flat_map(|c| c.givens.iter().map(|(q, _)| q.clone()))
        .filter(|q| !known.contains(q.as_str()))
        .collect();
    extra.sort_by_key(|q| store::qid_order(q));
    extra.dedup();
    println!("pool given names below that count: {}", extra.len());
    let rare = qlever::given_names_by_id(client, &extra)?;
    for g in common.iter().chain(rare.iter()) {
        graph.add_given_name(&GivenName {
            label: g.label.clone(),
            langs: g.langs.clone(),
            count: g.count,
        });
    }
    for c in candidates {
        for (_, display) in name_forms(c) {
            graph.ensure_form(&display);
        }
    }
    for p in &pool.people {
        for n in &p.names {
            graph.ensure_form(n);
        }
    }
    Ok(graph)
}

fn save(out: &Path, pool: &mut Pool, state: &State, today: &str) -> Result<()> {
    pool.generated = today.to_string();
    pool.sort();
    store::write_json(&out.join("pool.json"), pool)?;
    let state_out = State {
        version: 1,
        last_run: today.to_string(),
        processed: state.processed.clone(),
        skipped: state.skipped.clone(),
    };
    store::write_json(&out.join("state.json"), &state_out)?;
    Ok(())
}

fn append_pending(out: &Path, keys: &[String]) -> Result<()> {
    use std::io::Write;
    if keys.is_empty() {
        return Ok(());
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(out.join(PENDING_UPLOADS))?;
    for k in keys {
        writeln!(f, "{k}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate() -> Candidate {
        Candidate {
            qid: "Q7251".into(),
            label: "Alan Turing".into(),
            description: None,
            file: "Alan Turing (1951) (crop).jpg".into(),
            born: 1912,
            wiki: Some("Alan Turing".into()),
            givens: vec![
                ("Q18001787".into(), "Mathison".into()),
                ("Q4723915".into(), "Alan".into()),
            ],
            nicknames: vec!["Prof".into()],
            citizenship: vec!["United Kingdom".into()],
            occupations: vec![],
        }
    }

    #[test]
    fn display_prefers_the_given_name_matching_the_label() {
        assert_eq!(display_name(&candidate()), "Alan");
        let mut c = candidate();
        c.label = "Turing".into();
        assert_eq!(display_name(&c), "Mathison");
        c.givens.clear();
        assert_eq!(display_name(&c), "Turing");
    }

    #[test]
    fn name_forms_are_sorted_unique_normalized() {
        let forms: Vec<String> = name_forms(&candidate())
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(forms, vec!["alan", "mathison", "prof"]);
    }
}
