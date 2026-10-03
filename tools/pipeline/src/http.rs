use anyhow::Result;
use reqwest::blocking::{Client as Inner, RequestBuilder, Response};
use reqwest::StatusCode;
use std::thread::sleep;
use std::time::{Duration, Instant};

pub const USER_AGENT: &str =
    "whom-pipeline/0.1 (https://github.com/sean-reid/whom; seanreid.mail@gmail.com)";
pub const WIKIMEDIA_GAP: Duration = Duration::from_millis(1500);
pub const QLEVER_GAP: Duration = Duration::from_millis(1000);
pub const CLOUDFLARE_GAP: Duration = Duration::from_millis(300);
const BACKOFF_START: Duration = Duration::from_secs(30);
const MAX_429: u32 = 3;
const MAX_TRANSIENT_ATTEMPTS: u32 = 3;

#[derive(Debug)]
pub enum FetchError {
    NotFound,
    Transient(String),
    Fatal(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::NotFound => write!(f, "not found"),
            FetchError::Transient(s) => write!(f, "transient: {s}"),
            FetchError::Fatal(s) => write!(f, "fatal: {s}"),
        }
    }
}

impl std::error::Error for FetchError {}

struct Pacer {
    gap: Duration,
    last: Option<Instant>,
}

impl Pacer {
    fn wait(&mut self) {
        if let Some(last) = self.last {
            let due = last + self.gap;
            let now = Instant::now();
            if due > now {
                sleep(due - now);
            }
        }
        self.last = Some(Instant::now());
    }
}

pub struct Client {
    inner: Inner,
    wikimedia: Pacer,
    qlever: Pacer,
    cloudflare: Pacer,
    other: Pacer,
    consecutive_429: u32,
    pub requests: u64,
}

impl Client {
    pub fn new() -> Result<Self> {
        let inner = Inner::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(300))
            .build()?;
        let pacer = |gap| Pacer { gap, last: None };
        Ok(Client {
            inner,
            wikimedia: pacer(WIKIMEDIA_GAP),
            qlever: pacer(QLEVER_GAP),
            cloudflare: pacer(CLOUDFLARE_GAP),
            other: pacer(Duration::from_secs(1)),
            consecutive_429: 0,
            requests: 0,
        })
    }

    fn pacer_for(&mut self, url: &str) -> &mut Pacer {
        let host = url
            .split("://")
            .nth(1)
            .and_then(|r| r.split('/').next())
            .unwrap_or("");
        if host.ends_with("wikimedia.org") || host.ends_with("wikipedia.org") {
            &mut self.wikimedia
        } else if host.ends_with("qlever.dev") {
            &mut self.qlever
        } else if host.ends_with("cloudflare.com") {
            &mut self.cloudflare
        } else {
            &mut self.other
        }
    }

    pub fn get(&mut self, url: &str, accept: Option<&str>) -> Result<Response, FetchError> {
        self.send(url, |c| {
            let r = c.get(url);
            match accept {
                Some(a) => r.header("Accept", a),
                None => r,
            }
        })
    }

    pub fn send<F>(&mut self, url: &str, build: F) -> Result<Response, FetchError>
    where
        F: Fn(&Inner) -> RequestBuilder,
    {
        let mut transient_attempts = 0;
        let mut backoff = BACKOFF_START;
        loop {
            self.pacer_for(url).wait();
            self.requests += 1;
            let result = build(&self.inner).send();
            let resp = match result {
                Ok(r) => r,
                Err(e) => {
                    transient_attempts += 1;
                    if transient_attempts >= MAX_TRANSIENT_ATTEMPTS {
                        return Err(FetchError::Transient(e.to_string()));
                    }
                    eprintln!("  {url}: {e}; retrying in 5s");
                    sleep(Duration::from_secs(5));
                    continue;
                }
            };
            let status = resp.status();
            if status.is_success() {
                self.consecutive_429 = 0;
                return Ok(resp);
            }
            if status == StatusCode::TOO_MANY_REQUESTS || status == StatusCode::SERVICE_UNAVAILABLE
            {
                if status == StatusCode::TOO_MANY_REQUESTS {
                    self.consecutive_429 += 1;
                    if self.consecutive_429 >= MAX_429 {
                        return Err(FetchError::Fatal(format!(
                            "{MAX_429} consecutive 429 responses, last from {url}"
                        )));
                    }
                } else {
                    transient_attempts += 1;
                    if transient_attempts >= MAX_TRANSIENT_ATTEMPTS {
                        return Err(FetchError::Transient(format!("{status} from {url}")));
                    }
                }
                let wait = retry_after(&resp).unwrap_or(backoff);
                backoff *= 2;
                eprintln!("  {status} from {url}; waiting {}s", wait.as_secs());
                sleep(wait);
                continue;
            }
            if status == StatusCode::NOT_FOUND {
                return Err(FetchError::NotFound);
            }
            if status.is_server_error() {
                transient_attempts += 1;
                if transient_attempts >= MAX_TRANSIENT_ATTEMPTS {
                    return Err(FetchError::Transient(format!("{status} from {url}")));
                }
                sleep(backoff);
                backoff *= 2;
                continue;
            }
            let body = resp.text().unwrap_or_default();
            return Err(FetchError::Fatal(format!(
                "{status} from {url}: {}",
                body.chars().take(300).collect::<String>()
            )));
        }
    }
}

fn retry_after(resp: &Response) -> Option<Duration> {
    let v = resp.headers().get("retry-after")?.to_str().ok()?;
    let secs: u64 = v.trim().parse().ok()?;
    Some(Duration::from_secs(secs.max(1)))
}
