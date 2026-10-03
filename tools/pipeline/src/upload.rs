use crate::run::PENDING_UPLOADS;
use anyhow::{Context, Result};
use std::path::Path;

pub const RUN_OK: &str = "run-ok";
pub const MANIFESTS: [&str; 3] = ["pool.json", "names.json", "state.json"];

pub struct Plan {
    pub keys: Vec<String>,
    pub problems: Vec<String>,
}

impl Plan {
    pub fn ok(&self) -> bool {
        self.problems.is_empty()
    }
}

pub fn clear_marker(out: &Path) -> Result<()> {
    match std::fs::remove_file(out.join(RUN_OK)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("remove {RUN_OK}")),
    }
}

pub fn write_marker(out: &Path) -> Result<()> {
    std::fs::write(out.join(RUN_OK), "").with_context(|| format!("write {RUN_OK}"))
}

// The three manifests only make sense together, so a run that did not finish
// publishes crops alone and the manifests wait for the next good run.
pub fn plan(out: &Path) -> Plan {
    let mut keys = pending_keys(out);
    keys.retain(|k| !MANIFESTS.contains(&k.as_str()));
    keys.sort();
    keys.dedup();
    let mut problems = Vec::new();
    if !out.join(RUN_OK).exists() {
        problems.push(format!(
            "{RUN_OK} is missing, so the run did not finish; uploading crops only"
        ));
    } else {
        let missing: Vec<&str> = MANIFESTS
            .iter()
            .copied()
            .filter(|m| !out.join(m).exists())
            .collect();
        if missing.is_empty() {
            keys.extend(MANIFESTS.iter().map(|m| m.to_string()));
        } else {
            problems.push(format!(
                "{} missing; uploading crops only",
                missing.join(" and ")
            ));
        }
    }
    Plan { keys, problems }
}

fn pending_keys(out: &Path) -> Vec<String> {
    match std::fs::read_to_string(out.join(PENDING_UPLOADS)) {
        Ok(text) => text
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect(),
        Err(_) => std::fs::read_dir(out.join("crops"))
            .map(|rd| {
                rd.flatten()
                    .filter_map(|e| e.file_name().into_string().ok())
                    .filter(|n| n.ends_with(".jpg"))
                    .map(|n| format!("crops/{n}"))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct Out(PathBuf);

    impl Out {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("whom-upload-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("crops")).unwrap();
            Out(dir)
        }

        fn touch(&self, rel: &str) -> &Self {
            std::fs::write(self.0.join(rel), "x").unwrap();
            self
        }
    }

    impl Drop for Out {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn without_the_marker_only_crops_go_up() {
        let out = Out::new("no-marker");
        out.touch("crops/Q1.jpg")
            .touch("crops/Q2.jpg")
            .touch("pool.json")
            .touch("state.json");
        let plan = plan(&out.0);
        assert_eq!(plan.keys, ["crops/Q1.jpg", "crops/Q2.jpg"]);
        assert!(!plan.ok());
        assert_eq!(plan.problems.len(), 1);
        assert!(plan.problems[0].contains(RUN_OK), "{}", plan.problems[0]);
    }

    #[test]
    fn with_the_marker_and_every_manifest_everything_goes_up() {
        let out = Out::new("complete");
        out.touch("crops/Q1.jpg")
            .touch("pool.json")
            .touch("names.json")
            .touch("state.json");
        write_marker(&out.0).unwrap();
        let plan = plan(&out.0);
        assert_eq!(
            plan.keys,
            ["crops/Q1.jpg", "pool.json", "names.json", "state.json"]
        );
        assert!(plan.ok());
    }

    #[test]
    fn a_missing_manifest_holds_back_the_other_two() {
        let out = Out::new("missing-names");
        out.touch("crops/Q1.jpg")
            .touch("pool.json")
            .touch("state.json");
        write_marker(&out.0).unwrap();
        let plan = plan(&out.0);
        assert_eq!(plan.keys, ["crops/Q1.jpg"]);
        assert!(!plan.ok());
        assert_eq!(plan.problems, ["names.json missing; uploading crops only"]);
    }

    #[test]
    fn pending_list_wins_over_the_crops_dir_and_manifests_appear_once() {
        let out = Out::new("pending");
        out.touch("crops/Q1.jpg")
            .touch("crops/Q2.jpg")
            .touch("pool.json")
            .touch("names.json")
            .touch("state.json");
        std::fs::write(
            out.0.join(PENDING_UPLOADS),
            "crops/Q2.jpg\nnames.json\ncrops/Q2.jpg\n",
        )
        .unwrap();
        write_marker(&out.0).unwrap();
        let plan = plan(&out.0);
        assert_eq!(
            plan.keys,
            ["crops/Q2.jpg", "pool.json", "names.json", "state.json"]
        );
    }

    #[test]
    fn marker_is_cleared_and_written() {
        let out = Out::new("marker");
        clear_marker(&out.0).unwrap();
        assert!(!out.0.join(RUN_OK).exists());
        write_marker(&out.0).unwrap();
        assert!(out.0.join(RUN_OK).exists());
        clear_marker(&out.0).unwrap();
        assert!(!out.0.join(RUN_OK).exists());
    }
}
