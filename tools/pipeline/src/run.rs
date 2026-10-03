use crate::commons::{self, FileStatus};
use crate::face::{self, FaceDetector, Outcome};
use crate::http::{Client, FetchError};
use crate::names::Graph;
use crate::qlever::{self, Candidate};
use crate::store::{self, ImageInfo, Names, Person, Pool, Skip, State};
use crate::text::{first_token, is_name_label, normalize};
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
    skipped: BTreeMap<&'static str, usize>,
}

impl Report {
    fn skip(&mut self, reason: Skip) {
        *self.skipped.entry(reason.as_str()).or_insert(0) += 1;
    }

    fn print(&self, pool: &Pool, names: &Names, graph: &Graph, client: &Client, started: Instant) {
        println!();
        println!("people scanned: {}", self.scanned);
        println!("people needing a fetch: {}", self.to_fetch);
        println!("people added: {}", self.fetched);
        println!(
            "pool size: {} ({} retired)",
            pool.people.len(),
            pool.people.iter().filter(|p| p.retired).count()
        );
        println!("names in graph: {}", names.names.len());
        println!(
            "names with an era: {}, with sameAs: {}, with shortOf: {}",
            names.names.values().filter(|n| n.era.is_some()).count(),
            names
                .names
                .values()
                .filter(|n| !n.same_as.is_empty())
                .count(),
            names
                .names
                .values()
                .filter(|n| !n.short_of.is_empty())
                .count()
        );
        println!("given names without an English label: {}", graph.unlabelled);
        println!(
            "given-name labels failing the name rule: {}",
            graph.rejected_labels
        );
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
        for (reason, n) in &self.skipped {
            println!("  {reason}: {n}");
        }
        println!("requests: {}", client.requests);
        println!("wall time: {:.0}s", started.elapsed().as_secs_f64());
    }
}

pub fn crop_key(qid: &str) -> String {
    format!("crops/{qid}.jpg")
}

pub fn today() -> String {
    time::OffsetDateTime::now_utc().date().to_string()
}

// A scan smaller than nine tenths of the pool is a truncated QLever answer, not a mass
// departure of famous people, so it must not retire anyone.
pub fn scan_is_complete(scanned: usize, pool_size: usize) -> bool {
    scanned * 10 >= pool_size * 9
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
        if scan_is_complete(candidates.len(), pool.people.len()) {
            for p in &mut pool.people {
                p.retired = !seen.contains(p.qid.as_str());
            }
        } else {
            println!(
                "scan returned {} people against a pool of {}; skipping retirement",
                candidates.len(),
                pool.people.len()
            );
        }
    }
    for c in &candidates {
        if let Some(p) = pool.people.iter_mut().find(|p| p.qid == c.qid) {
            p.update_from(c);
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
            state.mark_skipped(&c.qid, Skip::NoLabel);
            report.skip(Skip::NoLabel);
        } else if !c.givens.iter().any(|(_, l)| is_name_label(l.trim())) {
            state.mark_skipped(&c.qid, Skip::NoGivenName);
            report.skip(Skip::NoGivenName);
        } else {
            with_names.push(c);
        }
    }

    let mut pending = Vec::new();
    let mut detector = FaceDetector::new()?;
    let mut fetched = 0;
    let mut handled = 0;
    let outcome = (|| -> Result<()> {
        for chunk in with_names.chunks(commons::BATCH) {
            if fetched >= args.max_fetch {
                break;
            }
            let files: Vec<String> = chunk.iter().map(|c| c.file.clone()).collect();
            let licences = commons::licences(&mut client, &files)?;
            for c in chunk {
                handled += 1;
                if handled % SAVE_EVERY == 0 {
                    save(&args.out, &mut pool, &state, &today)?;
                }
                let licence = match licences.get(&c.file) {
                    Some(FileStatus::Licensed(l)) => l.clone(),
                    Some(FileStatus::Rejected(reason)) => {
                        state.mark_skipped(&c.qid, *reason);
                        report.skip(*reason);
                        continue;
                    }
                    Some(FileStatus::Missing) | None => {
                        state.mark_skipped(&c.qid, Skip::MissingFile);
                        report.skip(Skip::MissingFile);
                        continue;
                    }
                };
                if fetched >= args.max_fetch {
                    continue;
                }
                fetched += 1;
                let reason =
                    match fetch_and_crop(&mut client, &mut detector, c, &licence.thumb, &crops_dir)
                    {
                        Ok(()) => None,
                        Err(Step::Skip(reason)) => Some(reason),
                        Err(Step::Fatal(e)) => return Err(e),
                    };
                match reason {
                    Some(reason) => {
                        state.mark_skipped(&c.qid, reason);
                        report.skip(reason);
                    }
                    None => {
                        state.mark_processed(&c.qid);
                        report.fetched += 1;
                        pending.push(crop_key(&c.qid));
                        pool.upsert(person_from(c, &licence));
                        println!("  {} {} ok", c.qid, c.label);
                    }
                }
            }
        }
        Ok(())
    })();
    save(&args.out, &mut pool, &state, &today)?;
    append_pending(&args.out, &pending)?;
    if let Err(e) = outcome {
        bail!("run stopped: {e}");
    }

    let mut graph = build_graph(&mut client, &candidates, &pool)?;
    let languages = qlever::labels(
        &mut client,
        &graph.langs_seen.iter().cloned().collect::<Vec<_>>(),
    )?;
    let names = Names {
        version: 1,
        languages,
        names: std::mem::take(&mut graph.names),
    };
    store::write_json(&args.out.join("names.json"), &names)?;
    append_pending(&args.out, &["names.json".to_string()])?;

    report.print(&pool, &names, &graph, &client, started);
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
    Skip(Skip),
    Fatal(anyhow::Error),
}

fn fetch_and_crop(
    client: &mut Client,
    detector: &mut FaceDetector,
    c: &Candidate,
    thumb_url: &str,
    crops_dir: &Path,
) -> Result<(), Step> {
    let thumb = match commons::fetch_thumb(client, thumb_url) {
        Ok(t) => t,
        Err(FetchError::NotFound) => return Err(Step::Skip(Skip::MissingFile)),
        Err(FetchError::Transient(e)) => {
            eprintln!("  {} {e}", c.qid);
            return Err(Step::Skip(Skip::FetchError));
        }
        Err(e @ FetchError::Fatal(_)) => return Err(Step::Fatal(e.into())),
    };
    let Some(img) = face::decode(&thumb.content_type, &thumb.bytes) else {
        return Err(Step::Skip(Skip::Undecodable));
    };
    let found = match detector.detect(&img) {
        Outcome::One(f) => f,
        Outcome::None => return Err(Step::Skip(Skip::NoFace)),
        Outcome::Many(n) => {
            eprintln!("  {} {n} faces", c.qid);
            return Err(Step::Skip(Skip::ManyFaces));
        }
        Outcome::Small(w) => {
            eprintln!("  {} face {w} px wide", c.qid);
            return Err(Step::Skip(Skip::SmallFace));
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
        .filter(|(_, l)| is_name_label(l.trim()))
        .find(|(_, l)| normalize(l) == wanted)
        .or_else(|| c.givens.iter().find(|(_, l)| is_name_label(l.trim())))
        .map(|(_, l)| l.trim().to_string())
        .unwrap_or_else(|| token.to_string())
}

const MAX_NICKNAME_WORDS: usize = 2;

// A P1449 nickname counts as a name form only when it is short enough to
// be one; longer values are sobriquets.
pub fn is_nickname_form(s: &str) -> bool {
    is_name_label(s) && s.split_whitespace().count() <= MAX_NICKNAME_WORDS
}

// The label's first token stands in for a given name only for mononyms; for
// everyone else it is a title or a stage name, not an answer.
pub fn name_forms(c: &Candidate) -> Vec<(String, String)> {
    let mut forms: Vec<(String, String)> = Vec::new();
    let givens: Vec<&str> = c
        .givens
        .iter()
        .map(|(_, l)| l.trim())
        .filter(|l| is_name_label(l))
        .collect();
    let label_token = if givens.is_empty() {
        first_token(&c.label).filter(|t| is_name_label(t))
    } else {
        None
    };
    let sources = givens
        .iter()
        .copied()
        .chain(
            c.nicknames
                .iter()
                .map(|n| n.trim())
                .filter(|n| is_nickname_form(n)),
        )
        .chain(label_token);
    for s in sources {
        let n = normalize(s);
        if !forms.iter().any(|(k, _)| *k == n) {
            forms.push((n, s.trim().to_string()));
        }
    }
    forms.sort();
    forms
}

pub fn name_keys(c: &Candidate) -> Vec<String> {
    name_forms(c).into_iter().map(|(k, _)| k).collect()
}

impl Person {
    pub fn update_from(&mut self, c: &Candidate) {
        self.label = c.label.clone();
        self.display = display_name(c);
        self.names = name_keys(c);
        self.born = c.born;
        self.citizenship = c.citizenship.clone();
        self.occupations = c.occupations.clone();
        self.description = c.description.clone();
        self.wiki = c.wiki.clone();
    }
}

fn person_from(c: &Candidate, licence: &commons::Licence) -> Person {
    let mut p = Person {
        qid: c.qid.clone(),
        label: String::new(),
        display: String::new(),
        names: Vec::new(),
        born: 0,
        citizenship: Vec::new(),
        occupations: Vec::new(),
        description: None,
        wiki: None,
        crop: crop_key(&c.qid),
        image: ImageInfo {
            file: c.file.clone(),
            artist: licence.artist.clone(),
            licence: licence.short_name.clone(),
            licence_url: licence.url.clone(),
            page_url: commons::page_url(&c.file),
        },
        retired: false,
    };
    p.update_from(c);
    p
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
        graph.add_given_name(g);
    }
    let qids: Vec<String> = graph.key_of_qid.keys().cloned().collect();
    for row in qlever::birth_years(client, &qids)? {
        graph.add_birth_years(&row.qid, row.year, row.count);
    }
    graph.link();
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
        assert_eq!(name_keys(&candidate()), vec!["alan", "mathison", "prof"]);
    }

    #[test]
    fn nicknames_are_capped_at_two_words() {
        assert!(is_nickname_form("Bill"));
        assert!(is_nickname_form("Pelé"));
        assert!(is_nickname_form("Fed Express"));
        assert!(!is_nickname_form("a pequena notavel"));
        assert!(!is_nickname_form("The Prof (1950s)"));
        let mut c = candidate();
        c.nicknames = vec!["Pelé".into(), "a pequena notavel".into(), "Big Al".into()];
        assert_eq!(name_keys(&c), vec!["alan", "big al", "mathison", "pele"]);
    }

    #[test]
    fn forms_and_display_skip_labels_that_are_not_names() {
        let mut c = candidate();
        c.givens.insert(0, ("Q1".into(), ".".into()));
        c.nicknames = vec!["The Prof (1950s)".into()];
        assert_eq!(name_keys(&c), vec!["alan", "mathison"]);
        c.label = "Turing".into();
        assert_eq!(display_name(&c), "Mathison");
    }

    #[test]
    fn label_token_counts_only_for_mononyms() {
        let mut c = candidate();
        c.label = "Lady Gaga".into();
        c.givens = vec![("Q18069632".into(), "Stefani".into())];
        c.nicknames.clear();
        assert_eq!(name_keys(&c), vec!["stefani"]);
        assert_eq!(display_name(&c), "Stefani");
        c.label = "Pelé".into();
        c.givens.clear();
        assert_eq!(name_keys(&c), vec!["pele"]);
        assert_eq!(display_name(&c), "Pelé");
        c.givens = vec![("Q1".into(), ".".into())];
        assert_eq!(name_keys(&c), vec!["pele"]);
    }

    #[test]
    fn a_short_scan_does_not_retire() {
        assert!(scan_is_complete(0, 0));
        assert!(scan_is_complete(9000, 10000));
        assert!(!scan_is_complete(8999, 10000));
        assert!(!scan_is_complete(2000, 11000));
    }
}
