use crate::commons::{self, FileStatus};
use crate::face::{self, FaceDetector, Outcome};
use crate::http::{Client, FetchError};
use crate::names::Graph;
use crate::qlever::{self, Candidate};
use crate::store::{self, ImageInfo, Names, Person, Pool, Skip, State};
use crate::text::{first_token, is_name_label, keeps_form, normalize};
use crate::upload;
use anyhow::{bail, Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const PENDING_UPLOADS: &str = "pending-uploads.txt";
const SAVE_STATE_EVERY: usize = 25;
const SAVE_POOL_EVERY: usize = 200;

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
    forms_dropped: usize,
    people_without_forms: usize,
    title_cased_forms: usize,
    names_with_region: usize,
    unmapped_countries: BTreeMap<String, u64>,
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
        println!("stale name forms dropped: {}", self.forms_dropped);
        println!(
            "people with no valid form, left out of the graph: {}",
            self.people_without_forms
        );
        println!(
            "pool-only forms shown title-cased for want of a source: {}",
            self.title_cased_forms
        );
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
        println!("names with a region: {}", self.names_with_region);
        let mut unmapped: Vec<(&String, &u64)> = self.unmapped_countries.iter().collect();
        unmapped.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        println!(
            "countries without a subregion: {} [{}]",
            unmapped.len(),
            unmapped
                .iter()
                .take(10)
                .map(|(c, n)| format!("{c} {n}"))
                .collect::<Vec<_>>()
                .join(", ")
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
        println!("qlever requests: {}", client.qlever_requests());
        println!("wikimedia requests: {}", client.wikimedia_requests());
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
    upload::clear_marker(&args.out)?;

    let mut state: State =
        store::read_json(&args.out.join("state.json"))?.unwrap_or_else(|| State::empty(&today));
    let (mut pool, forms_dropped) = load_pool(&args.out, &today)?;
    let orphaned = state.reconcile(&pool);
    if orphaned > 0 {
        println!("{orphaned} processed people are missing from the pool; fetching them again");
    }
    let mut client = Client::new()?;
    let mut report = Report {
        forms_dropped,
        ..Report::default()
    };

    let candidates = scan_and_retire(
        &mut client,
        qlever::ENDPOINT,
        &state,
        &mut pool,
        args.sample,
    )?;
    report.scanned = candidates.len();
    println!("scanned {} people", candidates.len());

    let index = pool.index();
    for c in &candidates {
        if let Some(&i) = index.get(&c.qid) {
            pool.people[i].update_from(c);
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
                let (state_due, pool_due) = saves_due(handled);
                if state_due {
                    save_state(&args.out, &state, &today)?;
                }
                if pool_due {
                    save_pool(&args.out, &mut pool, &today)?;
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
    save_state(&args.out, &state, &today)?;
    save_pool(&args.out, &mut pool, &today)?;
    append_pending(&args.out, &pending)?;
    if let Err(e) = outcome {
        bail!("run stopped: {e}");
    }

    let mut graph = build_graph(&mut client, &candidates)?;
    let added = add_pool_forms(&mut graph, &pool);
    report.people_without_forms = added.people_without_forms;
    report.title_cased_forms = added.title_cased;
    let regions = graph.assign_regions();
    report.names_with_region = regions.names_with_region;
    report.unmapped_countries = std::mem::take(&mut graph.unmapped_countries);
    let languages = qlever::labels(
        &mut client,
        &graph.langs_seen.iter().cloned().collect::<Vec<_>>(),
    )?;
    let names = Names {
        version: 1,
        languages,
        regions: regions.regions,
        names: std::mem::take(&mut graph.names),
    };
    store::write_json(&args.out.join("names.json"), &names)?;
    append_pending(&args.out, &["names.json".to_string()])?;
    upload::write_marker(&args.out)?;

    report.print(&pool, &names, &graph, &client, started);
    Ok(())
}

// A sample run stops scanning early and so never retires anyone.
fn scan_and_retire(
    client: &mut Client,
    endpoint: &str,
    state: &State,
    pool: &mut Pool,
    sample: Option<usize>,
) -> Result<Vec<Candidate>> {
    let candidates = scan(client, endpoint, state, sample)?;
    if sample.is_none() {
        retire(pool, &candidates);
    }
    Ok(candidates)
}

fn retire(pool: &mut Pool, candidates: &[Candidate]) {
    if !scan_is_complete(candidates.len(), pool.people.len()) {
        println!(
            "scan returned {} people against a pool of {}; skipping retirement",
            candidates.len(),
            pool.people.len()
        );
        return;
    }
    let seen: BTreeSet<&str> = candidates.iter().map(|c| c.qid.as_str()).collect();
    for p in &mut pool.people {
        p.retired = !seen.contains(p.qid.as_str());
    }
}

fn scan(
    client: &mut Client,
    endpoint: &str,
    state: &State,
    sample: Option<usize>,
) -> Result<Vec<Candidate>> {
    let mut candidates = Vec::new();
    qlever::paged(client, endpoint, &qlever::pool_query(), |rows| {
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

// The given name whose label matches the label's first token, else the first
// given name; the token itself for a mononym.
fn display_given(c: &Candidate) -> Option<&str> {
    let wanted = normalize(first_token(&c.label).unwrap_or(""));
    let valid = || {
        c.givens
            .iter()
            .map(|(_, l)| l.trim())
            .filter(|l| keeps_form(l))
    };
    valid()
        .find(|l| normalize(l) == wanted)
        .or_else(|| valid().next())
}

pub fn display_name(c: &Candidate) -> String {
    display_given(c)
        .or_else(|| first_token(&c.label))
        .unwrap_or("")
        .to_string()
}

pub fn nickname_forms(c: &Candidate) -> impl Iterator<Item = &str> {
    c.nicknames
        .iter()
        .map(|n| n.trim())
        .filter(|n| keeps_form(n))
}

// A win needs the name the person goes by: the display given name, a one-word
// nickname, or the label token of a mononym. Other given names are not answers.
pub fn name_forms(c: &Candidate) -> Vec<(String, String)> {
    let mut forms: Vec<(String, String)> = Vec::new();
    let sources = display_given(c)
        .or_else(|| first_token(&c.label).filter(|t| keeps_form(t)))
        .into_iter()
        .chain(nickname_forms(c));
    for s in sources {
        let n = normalize(s);
        if !forms.iter().any(|(k, _)| *k == n) {
            forms.push((n, s.to_string()));
        }
    }
    forms.sort();
    forms
}

pub fn name_keys(c: &Candidate) -> Vec<String> {
    name_forms(c).into_iter().map(|(k, _)| k).collect()
}

// Every stored form passes through the current rules on load, so a pool written
// by an older binary cannot feed the graph; returns how many forms went.
fn load_pool(out: &Path, today: &str) -> Result<(Pool, usize)> {
    let mut pool: Pool =
        store::read_json(&out.join("pool.json"))?.unwrap_or_else(|| Pool::empty(today));
    let dropped = pool
        .people
        .iter_mut()
        .map(|p| {
            let dropped = p.revalidate();
            p.derive_form_displays();
            dropped
        })
        .sum();
    Ok((pool, dropped))
}

impl Person {
    // A record written before formDisplays existed still carries two cased
    // strings, the display and the label; nickname forms have no source left.
    pub fn derive_form_displays(&mut self) {
        let token = first_token(&self.label).unwrap_or("").to_string();
        for cased in [self.display.clone(), token] {
            let key = normalize(&cased);
            if !cased.is_empty() && self.names.contains(&key) {
                self.form_displays.entry(key).or_insert(cased);
            }
        }
    }

    // Without a nickname mark a stored form could be a second given name, so only
    // the display form and marked nicknames survive; a full scan restores the rest.
    pub fn revalidate(&mut self) -> usize {
        let before = self.names.len();
        self.names.retain(|n| keeps_form(n));
        if !self.names.contains(&normalize(&self.display)) {
            let token = first_token(&self.label).unwrap_or("");
            self.display = if self.names.is_empty() || self.names.contains(&normalize(token)) {
                token.to_string()
            } else {
                capitalize(&self.names[0])
            };
        }
        let display = normalize(&self.display);
        let nicknames = &self.nicknames;
        self.names
            .retain(|n| *n == display || nicknames.contains(n));
        let names = &self.names;
        self.form_displays.retain(|k, _| names.contains(k));
        self.nicknames.retain(|k| names.contains(k));
        before - self.names.len()
    }

    pub fn update_from(&mut self, c: &Candidate) {
        self.label = c.label.clone();
        self.display = display_name(c);
        let forms = name_forms(c);
        self.names = forms.iter().map(|(k, _)| k.clone()).collect();
        self.form_displays = forms.into_iter().collect();
        self.nicknames = nickname_forms(c).map(normalize).collect();
        self.nicknames.sort();
        self.nicknames.dedup();
        self.born = c.born;
        self.citizenship = c.citizenship.clone();
        self.occupations = c.occupations.clone();
        self.description = c.description.clone();
        self.wiki = c.wiki.clone();
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn person_from(c: &Candidate, licence: &commons::Licence) -> Person {
    let mut p = Person {
        qid: c.qid.clone(),
        label: String::new(),
        display: String::new(),
        names: Vec::new(),
        form_displays: BTreeMap::new(),
        nicknames: Vec::new(),
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

fn build_graph(client: &mut Client, candidates: &[Candidate]) -> Result<Graph> {
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
    for row in qlever::citizenships(client, &qids)? {
        graph.add_citizenship(&row.qid, &row.country, row.count);
    }
    graph.link();
    for c in candidates {
        for (_, display) in name_forms(c) {
            graph.ensure_form(&display);
        }
    }
    Ok(graph)
}

#[derive(Default, Debug, PartialEq)]
struct PoolForms {
    people_without_forms: usize,
    title_cased: usize,
}

// Stored forms pass the rule again here, so a pool written by another binary
// cannot reach names.json. A form with no given-name item shows as the cased
// text the person carries for it, else title-cased.
fn add_pool_forms(graph: &mut Graph, pool: &Pool) -> PoolForms {
    let mut out = PoolForms::default();
    for p in &pool.people {
        let mut any = false;
        for n in p.names.iter().filter(|n| keeps_form(n)) {
            any = true;
            if graph.names.contains_key(n) {
                continue;
            }
            let cased = match p.form_displays.get(n) {
                Some(d) => d.clone(),
                None if normalize(&p.display) == *n => p.display.clone(),
                None => {
                    out.title_cased += 1;
                    title_case(n)
                }
            };
            graph.ensure_form(&cased);
            for country in &p.citizenship {
                graph.add_citizenship_label(n, country);
            }
        }
        if !any {
            out.people_without_forms += 1;
        }
    }
    out
}

pub fn title_case(s: &str) -> String {
    s.split(' ')
        .map(|w| w.split('-').map(capitalize).collect::<Vec<_>>().join("-"))
        .collect::<Vec<_>>()
        .join(" ")
}

// pool.json is around 5 MB at full size, state.json a tenth of that.
fn saves_due(handled: usize) -> (bool, bool) {
    (
        handled.is_multiple_of(SAVE_STATE_EVERY),
        handled.is_multiple_of(SAVE_POOL_EVERY),
    )
}

fn save_pool(out: &Path, pool: &mut Pool, today: &str) -> Result<()> {
    pool.generated = today.to_string();
    pool.sort();
    store::write_json(&out.join("pool.json"), pool)
}

fn save_state(out: &Path, state: &State, today: &str) -> Result<()> {
    let state_out = State {
        version: 1,
        last_run: today.to_string(),
        processed: state.processed.clone(),
        skipped: state.skipped.clone(),
    };
    store::write_json(&out.join("state.json"), &state_out)
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
    use crate::http::testing::{response, serve};

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
        assert_eq!(name_keys(&candidate()), vec!["alan", "prof"]);
    }

    #[test]
    fn a_form_is_one_word_without_a_period() {
        for s in ["Pelé", "Bill", "Lula", "Jean-Paul", "O'Neil"] {
            assert!(keeps_form(s), "{s}");
        }
        for s in [
            "Tony Shalhoub",
            "A Diva dos Pés Descalços",
            "A Pequena Notável",
            "S.",
            "A.",
            "Big Al",
            "Fed Express",
            "Mary Ann",
            "",
        ] {
            assert!(!keeps_form(s), "{s}");
        }
        let mut c = candidate();
        c.nicknames = vec!["Pelé".into(), "a pequena notavel".into(), "Big Al".into()];
        assert_eq!(name_keys(&c), vec!["alan", "pele"]);
    }

    #[test]
    fn live_junk_strings_never_become_forms_or_the_display() {
        let mut c = candidate();
        c.label = "Tony Shalhoub".into();
        c.givens = vec![
            ("Q1".into(), "Tony Shalhoub".into()),
            ("Q2".into(), "Anthony".into()),
        ];
        c.nicknames = vec![
            "Tony Shalhoub".into(),
            "A Diva dos Pés Descalços".into(),
            "A Pequena Notável".into(),
            "S.".into(),
            "A.".into(),
        ];
        assert_eq!(name_keys(&c), vec!["anthony"]);
        assert_eq!(display_name(&c), "Anthony");
        c.label = "Pelé".into();
        c.givens.clear();
        c.nicknames.clear();
        assert_eq!(name_keys(&c), vec!["pele"]);
    }

    #[test]
    fn forms_and_display_skip_labels_that_are_not_names() {
        let mut c = candidate();
        c.givens.insert(0, ("Q1".into(), ".".into()));
        c.nicknames = vec!["The Prof (1950s)".into()];
        assert_eq!(name_keys(&c), vec!["alan"]);
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
    fn state_saves_every_25_and_the_pool_every_200() {
        assert_eq!(saves_due(24), (false, false));
        assert_eq!(saves_due(25), (true, false));
        assert_eq!(saves_due(175), (true, false));
        assert_eq!(saves_due(200), (true, true));
        assert_eq!(saves_due(401), (false, false));
    }

    fn stored(label: &str, display: &str, names: &[&str]) -> Person {
        let mut p = Person::stub("Q1");
        p.label = label.into();
        p.display = display.into();
        p.names = names.iter().map(|n| n.to_string()).collect();
        p
    }

    #[test]
    fn stale_forms_are_dropped_on_load_and_display_survives() {
        let dir = std::env::temp_dir().join(format!("whom-forms-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut pool = Pool::empty("2026-01-01");
        pool.people.push(stored(
            "Alan Turing",
            "Alan",
            &["a pequena notavel", "alan", "s.", "tony shalhoub"],
        ));
        pool.people.push(stored("Pelé", "Pelé", &["pele"]));
        store::write_json(&dir.join("pool.json"), &pool).unwrap();
        let (loaded, dropped) = load_pool(&dir, "2026-02-01").unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(dropped, 3);
        assert_eq!(loaded.people[0].names, ["alan"]);
        assert_eq!(loaded.people[0].display, "Alan");
        assert_eq!(loaded.people[1].names, ["pele"]);
        assert_eq!(loaded.people[1].display, "Pelé");
        let (empty, dropped) = load_pool(&dir, "2026-02-01").unwrap();
        assert_eq!((empty.people.len(), dropped), (0, 0));
    }

    #[test]
    fn a_stored_person_derives_cased_forms_from_display_and_label_on_load() {
        let mut p = stored("Antonín Novotný", "Tonda", &["antonin", "franta", "tonda"]);
        p.derive_form_displays();
        assert_eq!(
            p.form_displays,
            BTreeMap::from([
                ("antonin".to_string(), "Antonín".to_string()),
                ("tonda".to_string(), "Tonda".to_string()),
            ])
        );
        let mut pool = Pool::empty("2026-01-01");
        pool.people.push(p);
        let mut graph = Graph::new();
        assert_eq!(
            add_pool_forms(&mut graph, &pool),
            PoolForms {
                people_without_forms: 0,
                title_cased: 1
            }
        );
        assert_eq!(graph.names["antonin"].display, "Antonín");
        assert_eq!(graph.names["franta"].display, "Franta");
        let mut p = stored("Pelé", "Pelé", &["pele"]);
        p.form_displays.insert("pele".into(), "Pelé ".into());
        p.derive_form_displays();
        assert_eq!(p.form_displays["pele"], "Pelé ");
        let dir = std::env::temp_dir().join(format!("whom-cased-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut pool = Pool::empty("2026-01-01");
        pool.people
            .push(stored("Antonín Novotný", "Antonín", &["antonin"]));
        store::write_json(&dir.join("pool.json"), &pool).unwrap();
        let (loaded, _) = load_pool(&dir, "2026-02-01").unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(loaded.people[0].form_displays["antonin"], "Antonín");
    }

    #[test]
    fn only_the_name_the_person_goes_by_wins() {
        let mut c = candidate();
        c.label = "Benjamin Britten".into();
        c.givens = vec![
            ("Q4854186".into(), "Edward".into()),
            ("Q18002399".into(), "Benjamin".into()),
        ];
        c.nicknames.clear();
        assert_eq!(name_keys(&c), vec!["benjamin"]);
        assert_eq!(display_name(&c), "Benjamin");
        c.label = "Bill Clinton".into();
        c.givens = vec![
            ("Q12344159".into(), "William".into()),
            ("Q1158570".into(), "Jefferson".into()),
        ];
        c.nicknames = vec!["Bill".into(), "Slick Willie".into()];
        assert_eq!(name_keys(&c), vec!["bill", "william"]);
        assert_eq!(display_name(&c), "William");
        let mut p = Person::stub("Q1124");
        p.update_from(&c);
        assert_eq!(p.nicknames, ["bill"]);
        assert_eq!(p.form_displays["bill"], "Bill");
    }

    #[test]
    fn stored_people_keep_the_display_form_and_marked_nicknames() {
        let mut p = stored("Benjamin Britten", "Benjamin", &["benjamin", "edward"]);
        assert_eq!(p.revalidate(), 1);
        assert_eq!(p.names, ["benjamin"]);
        assert_eq!(p.display, "Benjamin");
        let mut p = stored("Pelé", "Pelé", &["pele"]);
        assert_eq!(p.revalidate(), 0);
        assert_eq!(p.names, ["pele"]);
        let mut p = stored("Bill Clinton", "William", &["bill", "jefferson", "william"]);
        p.form_displays.insert("bill".into(), "Bill".into());
        assert_eq!(p.revalidate(), 2);
        assert_eq!(p.names, ["william"]);
        assert!(p.form_displays.is_empty());
        let mut p = stored("Bill Clinton", "William", &["bill", "jefferson", "william"]);
        p.form_displays.insert("bill".into(), "Bill".into());
        p.nicknames = vec!["bill".into(), "slick willie".into()];
        assert_eq!(p.revalidate(), 1);
        assert_eq!(p.names, ["bill", "william"]);
        assert_eq!(p.nicknames, ["bill"]);
        assert_eq!(p.form_displays["bill"], "Bill");
    }

    #[test]
    fn display_is_rebuilt_only_when_its_form_was_dropped() {
        let mut p = stored("Lady Gaga", "Lady Gaga", &["lady gaga", "stefani"]);
        assert_eq!(p.revalidate(), 1);
        assert_eq!(p.names, ["stefani"]);
        assert_eq!(p.display, "Stefani");
        let mut p = stored("Alan Turing", "Mathison", &["alan", "mathison", "a."]);
        assert_eq!(p.revalidate(), 2);
        assert_eq!(p.display, "Mathison");
        assert_eq!(p.names, ["mathison"]);
        let mut p = stored("Alan Turing", "Tony Shalhoub", &["alan", "tony shalhoub"]);
        assert_eq!(p.revalidate(), 1);
        assert_eq!(p.display, "Alan");
    }

    #[test]
    fn update_from_keeps_the_cased_text_of_every_form() {
        let mut p = Person::stub("Q7251");
        p.update_from(&candidate());
        assert_eq!(p.names, ["alan", "prof"]);
        assert_eq!(p.nicknames, ["prof"]);
        assert_eq!(
            p.form_displays,
            BTreeMap::from([
                ("alan".to_string(), "Alan".to_string()),
                ("prof".to_string(), "Prof".to_string()),
            ])
        );
        let mut p = stored("Lady Gaga", "Lady Gaga", &["lady gaga", "stefani"]);
        p.form_displays
            .insert("lady gaga".into(), "Lady Gaga".into());
        p.form_displays.insert("stefani".into(), "Stefani".into());
        assert_eq!(p.revalidate(), 1);
        assert_eq!(
            p.form_displays,
            BTreeMap::from([("stefani".to_string(), "Stefani".to_string())])
        );
    }

    #[test]
    fn a_person_with_only_junk_forms_keeps_a_display_and_is_not_retired() {
        let mut p = stored("Tony Shalhoub", "Tony Shalhoub", &["tony shalhoub", "s."]);
        assert_eq!(p.revalidate(), 2);
        assert!(p.names.is_empty());
        assert_eq!(p.display, "Tony");
        assert!(!p.retired);
    }

    #[test]
    fn graph_takes_only_valid_stored_forms_and_counts_people_without_one() {
        let mut pool = Pool::empty("2026-01-01");
        pool.people.push(stored(
            "Alan Turing",
            "Alan",
            &["alan", "tony shalhoub", "s."],
        ));
        pool.people.push(stored("Tony Shalhoub", "Tony", &[]));
        pool.people
            .push(stored("Carmen Miranda", "Carmen", &["a pequena notavel"]));
        let mut graph = Graph::new();
        assert_eq!(
            add_pool_forms(&mut graph, &pool),
            PoolForms {
                people_without_forms: 2,
                title_cased: 0
            }
        );
        let keys: Vec<&String> = graph.names.keys().collect();
        assert_eq!(keys, ["alan"]);
        assert_eq!(graph.names["alan"].display, "Alan");
        assert!(!pool.people[1].retired);
    }

    #[test]
    fn a_pool_only_person_gives_their_form_a_region_by_label() {
        let mut c = candidate();
        c.qid = "Q57621".into();
        c.label = "Hifikepunye Pohamba".into();
        c.givens.clear();
        c.nicknames.clear();
        c.citizenship = vec!["Namibia".into(), "South West Africa".into()];
        let mut p = Person::stub(&c.qid);
        p.update_from(&c);
        let mut pool = Pool::empty("2026-01-01");
        pool.people.push(p);
        let mut graph = Graph::new();
        add_pool_forms(&mut graph, &pool);
        let stats = graph.assign_regions();
        assert_eq!(stats.names_with_region, 1);
        assert_eq!(stats.regions["southern-africa"], "Southern Africa");
        assert_eq!(
            graph.names["hifikepunye"].region.as_deref(),
            Some("southern-africa")
        );
        assert_eq!(graph.names["hifikepunye"].region_share, Some(1.0));
        assert_eq!(graph.unmapped_countries["South West Africa"], 1);
    }

    #[test]
    fn a_label_only_form_keeps_its_case_in_the_graph() {
        let mut c = candidate();
        c.qid = "Q57621".into();
        c.label = "Hifikepunye Pohamba".into();
        c.givens.clear();
        c.nicknames.clear();
        let mut p = Person::stub(&c.qid);
        p.update_from(&c);
        assert_eq!(p.names, ["hifikepunye"]);
        assert_eq!(p.form_displays["hifikepunye"], "Hifikepunye");
        let mut pool = Pool::empty("2026-01-01");
        pool.people.push(p);
        let mut graph = Graph::new();
        assert_eq!(add_pool_forms(&mut graph, &pool), PoolForms::default());
        assert_eq!(graph.names["hifikepunye"].display, "Hifikepunye");
    }

    #[test]
    fn a_stored_person_without_cased_forms_falls_back_to_display_then_title_case() {
        let mut pool = Pool::empty("2026-01-01");
        pool.people.push(stored(
            "Antonín Novotný",
            "Antonín",
            &["antonin", "jean-paul", "tonda"],
        ));
        let mut graph = Graph::new();
        graph.ensure_form("Tonda");
        assert_eq!(
            add_pool_forms(&mut graph, &pool),
            PoolForms {
                people_without_forms: 0,
                title_cased: 1
            }
        );
        assert_eq!(graph.names["antonin"].display, "Antonín");
        assert_eq!(graph.names["jean-paul"].display, "Jean-Paul");
        assert_eq!(graph.names["tonda"].display, "Tonda");
    }

    #[test]
    fn title_case_uppercases_each_word_and_hyphen_part() {
        assert_eq!(title_case("hifikepunye"), "Hifikepunye");
        assert_eq!(title_case("jean-paul"), "Jean-Paul");
        assert_eq!(title_case("mary ann"), "Mary Ann");
        assert_eq!(title_case("o'neil"), "O'neil");
    }

    #[test]
    fn a_short_scan_does_not_retire() {
        assert!(scan_is_complete(0, 0));
        assert!(scan_is_complete(9000, 10000));
        assert!(!scan_is_complete(8999, 10000));
        assert!(!scan_is_complete(2000, 11000));
        let mut pool = Pool::empty("2026-01-01");
        pool.people
            .extend((1..=3000).map(|i| Person::stub(&format!("Q{i}"))));
        let candidates: Vec<Candidate> = (1..=2003).map(candidate_n).collect();
        retire(&mut pool, &candidates);
        assert!(pool.people.iter().all(|p| !p.retired));
    }

    fn candidate_n(i: usize) -> Candidate {
        let mut c = candidate();
        c.qid = format!("Q{i}");
        c
    }

    fn page(qids: std::ops::Range<usize>) -> String {
        let rows: Vec<String> = qids
            .map(|i| {
                format!(
                    concat!(
                        r#"{{"p":{{"value":"http://www.wikidata.org/entity/Q{i}"}},"#,
                        r#""label":{{"value":"Person {i}"}},"#,
                        r#""img":{{"value":"http://commons.wikimedia.org/wiki/Special:FilePath/Q{i}.jpg"}},"#,
                        r#""dob":{{"value":"1950-01-01T00:00:00Z"}}}}"#
                    ),
                    i = i
                )
            })
            .collect();
        response(
            "200 OK",
            &[("Content-Type", "application/sparql-results+json")],
            &format!(r#"{{"results":{{"bindings":[{}]}}}}"#, rows.join(",")),
        )
    }

    #[test]
    fn a_full_scan_pages_until_a_short_page_and_retires_the_unseen() {
        let (base, handle) = serve(vec![page(1..2001), page(2001..2004)]);
        let mut client = Client::new().unwrap();
        let state = State::empty("2026-01-01");
        let mut pool = Pool::empty("2026-01-01");
        pool.people
            .extend(["Q1", "Q2003", "Q5000"].map(Person::stub));
        let candidates = scan_and_retire(&mut client, &base, &state, &mut pool, None).unwrap();
        assert_eq!(candidates.len(), 2003);
        assert_eq!(candidates[0].qid, "Q1");
        assert_eq!(candidates[2002].label, "Person 2003");
        assert_eq!(client.requests, 2);
        let seen = handle.join().unwrap();
        assert_eq!(seen[0].line, "POST / HTTP/1.1");
        assert!(
            seen[0].body.starts_with("query=PREFIX+wd"),
            "{}",
            seen[0].body
        );
        assert!(
            seen[0].body.ends_with("LIMIT+2000+OFFSET+0"),
            "{}",
            seen[0].body
        );
        assert!(
            seen[1].body.ends_with("LIMIT+2000+OFFSET+2000"),
            "{}",
            seen[1].body
        );
        let retired: Vec<&str> = pool
            .people
            .iter()
            .filter(|p| p.retired)
            .map(|p| p.qid.as_str())
            .collect();
        assert_eq!(retired, ["Q5000"]);
    }

    #[test]
    fn a_sample_scan_stops_early_and_retires_nobody() {
        let (base, handle) = serve(vec![page(1..2001)]);
        let mut client = Client::new().unwrap();
        let state = State::empty("2026-01-01");
        let mut pool = Pool::empty("2026-01-01");
        pool.people.push(Person::stub("Q5000"));
        let candidates = scan_and_retire(&mut client, &base, &state, &mut pool, Some(5)).unwrap();
        assert_eq!(candidates.len(), 2000);
        assert_eq!(client.requests, 1);
        assert!(!pool.people[0].retired);
        assert_eq!(handle.join().unwrap().len(), 1);
    }

    #[test]
    fn a_failed_page_retires_nobody() {
        let (base, handle) = serve(vec![
            page(1..2001),
            response("200 OK", &[], r#"{"results":{}}"#),
        ]);
        let mut client = Client::new().unwrap();
        let state = State::empty("2026-01-01");
        let mut pool = Pool::empty("2026-01-01");
        pool.people.push(Person::stub("Q5000"));
        let err = scan_and_retire(&mut client, &base, &state, &mut pool, None).unwrap_err();
        assert!(err.to_string().contains("without bindings"), "{err}");
        assert_eq!(client.requests, 2);
        assert!(!pool.people[0].retired);
        assert_eq!(handle.join().unwrap().len(), 2);
    }
}
