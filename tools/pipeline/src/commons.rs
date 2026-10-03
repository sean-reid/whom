use crate::http::{Client, FetchError};
use crate::licence;
use crate::text::strip_html;
use anyhow::{anyhow, Context, Result};
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde_json::Value;
use std::collections::BTreeMap;

pub const API: &str = "https://commons.wikimedia.org/w/api.php";
pub const BATCH: usize = 50;
pub const THUMB_WIDTH: u32 = 1280;

const PATH_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~')
    .remove(b'(')
    .remove(b')')
    .remove(b',')
    .remove(b'!');

#[derive(Debug, Clone, PartialEq)]
pub struct Licence {
    pub short_name: String,
    pub url: Option<String>,
    pub artist: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileStatus {
    Licensed(Licence),
    Rejected(String),
    Missing,
}

pub fn page_url(file: &str) -> String {
    format!(
        "https://commons.wikimedia.org/wiki/File:{}",
        utf8_percent_encode(&file.replace(' ', "_"), PATH_SET)
    )
}

pub fn thumb_url(file: &str) -> String {
    format!(
        "https://commons.wikimedia.org/wiki/Special:FilePath/{}?width={THUMB_WIDTH}",
        utf8_percent_encode(file, PATH_SET)
    )
}

pub fn canonical_title(file: &str) -> String {
    let mut t = file.trim().replace('_', " ");
    if let Some(first) = t.chars().next() {
        if first.is_lowercase() {
            let upper: String = first.to_uppercase().collect();
            t.replace_range(..first.len_utf8(), &upper);
        }
    }
    t
}

pub fn licences(client: &mut Client, files: &[String]) -> Result<BTreeMap<String, FileStatus>> {
    let mut out = BTreeMap::new();
    for chunk in files.chunks(BATCH) {
        let titles: Vec<String> = chunk
            .iter()
            .map(|f| format!("File:{}", canonical_title(f)))
            .collect();
        let joined = titles.join("|");
        let resp = client
            .send(API, |c, u| {
                c.get(u).query(&[
                    ("action", "query"),
                    ("prop", "imageinfo"),
                    ("iiprop", "extmetadata"),
                    ("format", "json"),
                    ("formatversion", "2"),
                    ("titles", joined.as_str()),
                ])
            })
            .map_err(|e| anyhow!("imageinfo: {e}"))?;
        let body: Value = resp.json().context("imageinfo json")?;
        let parsed = parse_imageinfo(&body);
        for f in chunk {
            let status = parsed
                .get(&canonical_title(f))
                .cloned()
                .unwrap_or(FileStatus::Missing);
            out.insert(f.clone(), status);
        }
    }
    Ok(out)
}

pub fn parse_imageinfo(body: &Value) -> BTreeMap<String, FileStatus> {
    let mut by_title = BTreeMap::new();
    let mut aliases: BTreeMap<String, String> = BTreeMap::new();
    for kind in ["normalized", "redirects"] {
        if let Some(list) = body
            .pointer(&format!("/query/{kind}"))
            .and_then(Value::as_array)
        {
            for e in list {
                if let (Some(from), Some(to)) = (
                    e.get("from").and_then(Value::as_str),
                    e.get("to").and_then(Value::as_str),
                ) {
                    aliases.insert(to.to_string(), from.to_string());
                }
            }
        }
    }
    let Some(pages) = body.pointer("/query/pages").and_then(Value::as_array) else {
        return by_title;
    };
    for page in pages {
        let Some(title) = page.get("title").and_then(Value::as_str) else {
            continue;
        };
        let status = if page.get("missing").is_some() {
            FileStatus::Missing
        } else {
            match page.pointer("/imageinfo/0/extmetadata") {
                Some(meta) => status_from_metadata(meta),
                None => FileStatus::Missing,
            }
        };
        let mut key = title.strip_prefix("File:").unwrap_or(title).to_string();
        if let Some(original) = aliases.get(title) {
            key = original
                .strip_prefix("File:")
                .unwrap_or(original)
                .to_string();
        }
        by_title.insert(key, status);
    }
    by_title
}

fn meta_value<'a>(meta: &'a Value, key: &str) -> Option<&'a str> {
    meta.pointer(&format!("/{key}/value"))
        .and_then(Value::as_str)
}

fn status_from_metadata(meta: &Value) -> FileStatus {
    let Some(short) = meta_value(meta, "LicenseShortName")
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return FileStatus::Rejected("licence".into());
    };
    if !licence::accepted(short) {
        return FileStatus::Rejected("licence".into());
    }
    let artist = meta_value(meta, "Artist")
        .map(strip_html)
        .filter(|s| !s.is_empty());
    FileStatus::Licensed(Licence {
        short_name: short.to_string(),
        url: meta_value(meta, "LicenseUrl")
            .map(str::to_string)
            .filter(|s| !s.is_empty()),
        artist,
    })
}

pub struct Thumb {
    pub content_type: String,
    pub bytes: Vec<u8>,
}

pub fn fetch_thumb(client: &mut Client, file: &str) -> Result<Thumb, FetchError> {
    let url = thumb_url(file);
    let resp = client.get(&url, None)?;
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = resp
        .bytes()
        .map_err(|e| FetchError::Transient(e.to_string()))?
        .to_vec();
    Ok(Thumb {
        content_type,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn urls_are_percent_encoded() {
        assert_eq!(
            thumb_url("Antonín Novotný 1968.jpg"),
            "https://commons.wikimedia.org/wiki/Special:FilePath/Anton%C3%ADn%20Novotn%C3%BD%201968.jpg?width=1280"
        );
        assert_eq!(
            page_url("Alan Turing (1951) (crop).jpg"),
            "https://commons.wikimedia.org/wiki/File:Alan_Turing_(1951)_(crop).jpg"
        );
    }

    #[test]
    fn canonical_title_uppercases_first_letter_and_drops_underscores() {
        assert_eq!(canonical_title("alan_turing.jpg"), "Alan turing.jpg");
        assert_eq!(canonical_title("Émile.jpg"), "Émile.jpg");
    }

    #[test]
    fn imageinfo_response_maps_titles_to_status() {
        let body = json!({
            "query": {
                "normalized": [{"from": "File:a b.jpg", "to": "File:A b.jpg"}],
                "pages": [
                    {"title": "File:A b.jpg", "imageinfo": [{"extmetadata": {
                        "LicenseShortName": {"value": "CC BY-SA 3.0 nl"},
                        "LicenseUrl": {"value": "https://creativecommons.org/licenses/by-sa/3.0/nl/deed.en"},
                        "Artist": {"value": "<a href=\"//x\">Anefo</a>"}
                    }}]},
                    {"title": "File:Fair.jpg", "imageinfo": [{"extmetadata": {
                        "LicenseShortName": {"value": "Fair use"}
                    }}]},
                    {"title": "File:None.jpg", "imageinfo": [{"extmetadata": {}}]},
                    {"title": "File:Gone.jpg", "missing": true}
                ]
            }
        });
        let m = parse_imageinfo(&body);
        assert_eq!(
            m["a b.jpg"],
            FileStatus::Licensed(Licence {
                short_name: "CC BY-SA 3.0 nl".into(),
                url: Some("https://creativecommons.org/licenses/by-sa/3.0/nl/deed.en".into()),
                artist: Some("Anefo".into()),
            })
        );
        assert_eq!(m["Fair.jpg"], FileStatus::Rejected("licence".into()));
        assert_eq!(m["None.jpg"], FileStatus::Rejected("licence".into()));
        assert_eq!(m["Gone.jpg"], FileStatus::Missing);
    }
}
