use crate::http::{Client, FetchError};
use anyhow::{anyhow, bail, Context, Result};
use std::path::Path;

pub const BUCKET: &str = "whom";

pub struct R2 {
    account: String,
    token: String,
}

impl R2 {
    pub fn from_env() -> Result<Self> {
        let account = std::env::var("CLOUDFLARE_ACCOUNT_ID").context("CLOUDFLARE_ACCOUNT_ID")?;
        let token = std::env::var("CLOUDFLARE_API_TOKEN").context("CLOUDFLARE_API_TOKEN")?;
        if account.is_empty() || token.is_empty() {
            bail!("CLOUDFLARE_ACCOUNT_ID and CLOUDFLARE_API_TOKEN must be set");
        }
        Ok(R2 { account, token })
    }

    fn url(&self, key: &str) -> String {
        format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/r2/buckets/{BUCKET}/objects/{key}",
            self.account
        )
    }

    pub fn put(
        &self,
        client: &mut Client,
        key: &str,
        body: Vec<u8>,
        content_type: &str,
    ) -> Result<()> {
        let url = self.url(key);
        client
            .send(&url, |c| {
                c.put(&url)
                    .bearer_auth(&self.token)
                    .header("Content-Type", content_type)
                    .body(body.clone())
            })
            .map_err(|e| anyhow!("put {key}: {e}"))?;
        Ok(())
    }

    pub fn get(&self, client: &mut Client, key: &str) -> Result<Option<Vec<u8>>> {
        let url = self.url(key);
        match client.send(&url, |c| c.get(&url).bearer_auth(&self.token)) {
            Ok(resp) => Ok(Some(resp.bytes().context("read body")?.to_vec())),
            Err(FetchError::NotFound) => Ok(None),
            Err(e) => Err(anyhow!("get {key}: {e}")),
        }
    }
}

pub fn content_type_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("json") => "application/json",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        _ => "application/octet-stream",
    }
}
