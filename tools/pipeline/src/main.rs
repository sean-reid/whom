use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use pipeline::{http, r2, run, upload};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "pipeline", about = "Builds the WHOM? face pool and name graph")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Scan Wikidata, fetch portraits, crop faces, build the name graph.
    Run {
        #[arg(long)]
        out: PathBuf,
        /// Most Commons image fetches in one run.
        #[arg(long, default_value_t = 10000)]
        max_fetch: usize,
        /// Stop after this many new people; skips retirement.
        #[arg(long)]
        sample: Option<usize>,
    },
    /// Push new crops to R2, plus pool.json, names.json, and state.json after a finished run.
    Upload {
        #[arg(long)]
        out: PathBuf,
    },
    /// Pull state and manifests from R2 before a run.
    Download {
        #[arg(long)]
        out: PathBuf,
    },
}

fn main() {
    if let Err(e) = real_main() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Run {
            out,
            max_fetch,
            sample,
        } => run::run(&run::Args {
            out,
            max_fetch,
            sample,
        }),
        Cmd::Upload { out } => upload(&out),
        Cmd::Download { out } => download(&out),
    }
}

fn upload(out: &Path) -> Result<()> {
    let bucket = r2::R2::from_env()?;
    let mut client = http::Client::new()?;
    let plan = upload::plan(out);
    for key in &plan.keys {
        let path = out.join(key);
        let body =
            std::fs::read(&path).with_context(|| format!("read {} for upload", path.display()))?;
        bucket.put(&mut client, key, body, r2::content_type_for(&path))?;
    }
    let _ = std::fs::remove_file(out.join(run::PENDING_UPLOADS));
    println!("uploaded {} objects", plan.keys.len());
    if !plan.ok() {
        bail!("{}", plan.problems.join("; "));
    }
    Ok(())
}

fn download(out: &Path) -> Result<()> {
    let bucket = r2::R2::from_env()?;
    let mut client = http::Client::new()?;
    std::fs::create_dir_all(out).context("create out dir")?;
    for key in upload::MANIFESTS {
        match bucket.get(&mut client, key)? {
            Some(body) => {
                std::fs::write(out.join(key), body)?;
                println!("downloaded {key}");
            }
            None => println!("{key} absent in R2, starting fresh"),
        }
    }
    Ok(())
}
