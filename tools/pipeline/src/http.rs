use anyhow::Result;
use reqwest::blocking::{Client as Inner, RequestBuilder, Response};
use reqwest::redirect::Policy;
use reqwest::{StatusCode, Url};
use std::time::{Duration, Instant};

pub const USER_AGENT: &str =
    "whom-pipeline/0.1 (https://github.com/sean-reid/whom; seanreid.mail@gmail.com)";
pub const WIKIMEDIA_GAP: Duration = Duration::from_millis(1500);
pub const QLEVER_GAP: Duration = Duration::from_millis(3000);
pub const CLOUDFLARE_GAP: Duration = Duration::from_millis(300);
const BACKOFF_START: Duration = Duration::from_secs(30);
const BACKOFF_429_START: Duration = Duration::from_secs(60);
const MAX_429: u32 = 5;
const MAX_TRANSIENT_ATTEMPTS: u32 = 3;
const MAX_REDIRECTS: u32 = 5;

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

pub trait Sleeper {
    fn sleep(&mut self, d: Duration);
}

pub struct ThreadSleeper;

impl Sleeper for ThreadSleeper {
    fn sleep(&mut self, d: Duration) {
        std::thread::sleep(d);
    }
}

struct Pacer {
    gap: Duration,
    last: Option<Instant>,
    requests: u64,
}

impl Pacer {
    fn wait(&mut self) {
        if let Some(last) = self.last {
            let due = last + self.gap;
            let now = Instant::now();
            if due > now {
                std::thread::sleep(due - now);
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
    sleeper: Box<dyn Sleeper>,
    consecutive_429: u32,
    pub requests: u64,
}

impl Client {
    pub fn new() -> Result<Self> {
        Self::with_sleeper(Box::new(ThreadSleeper))
    }

    // The pacer gaps always use the thread clock; only the backoff waits go
    // through the sleeper.
    pub fn with_sleeper(sleeper: Box<dyn Sleeper>) -> Result<Self> {
        let inner = Inner::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(300))
            .redirect(Policy::none())
            .build()?;
        let pacer = |gap| Pacer {
            gap,
            last: None,
            requests: 0,
        };
        Ok(Client {
            inner,
            wikimedia: pacer(WIKIMEDIA_GAP),
            qlever: pacer(QLEVER_GAP),
            cloudflare: pacer(CLOUDFLARE_GAP),
            other: pacer(Duration::from_secs(1)),
            sleeper,
            consecutive_429: 0,
            requests: 0,
        })
    }

    pub fn qlever_requests(&self) -> u64 {
        self.qlever.requests
    }

    pub fn wikimedia_requests(&self) -> u64 {
        self.wikimedia.requests
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
        self.send(url, |c, u| {
            let r = c.get(u);
            match accept {
                Some(a) => r.header("Accept", a),
                None => r,
            }
        })
    }

    // `build` receives the URL to hit, which changes when a 3xx is followed.
    pub fn send<F>(&mut self, url: &str, build: F) -> Result<Response, FetchError>
    where
        F: Fn(&Inner, &str) -> RequestBuilder,
    {
        let mut url = url.to_string();
        let mut transient_attempts = 0;
        let mut redirects = 0;
        let mut backoff = BACKOFF_START;
        loop {
            let pacer = self.pacer_for(&url);
            pacer.wait();
            pacer.requests += 1;
            self.requests += 1;
            let result = build(&self.inner, &url).send();
            let resp = match result {
                Ok(r) => r,
                Err(e) => {
                    transient_attempts += 1;
                    if transient_attempts >= MAX_TRANSIENT_ATTEMPTS {
                        return Err(FetchError::Transient(e.to_string()));
                    }
                    eprintln!("  {url}: {e}; retrying in 5s");
                    self.sleeper.sleep(Duration::from_secs(5));
                    continue;
                }
            };
            let status = resp.status();
            if status.is_success() {
                self.consecutive_429 = 0;
                return Ok(resp);
            }
            if status.is_redirection() {
                redirects += 1;
                if redirects > MAX_REDIRECTS {
                    return Err(FetchError::Fatal(format!(
                        "more than {MAX_REDIRECTS} redirects from {url}"
                    )));
                }
                url = redirect_target(&url, &resp).ok_or_else(|| {
                    FetchError::Fatal(format!("{status} without a usable Location from {url}"))
                })?;
                continue;
            }
            if status == StatusCode::TOO_MANY_REQUESTS {
                self.consecutive_429 += 1;
                if self.consecutive_429 >= MAX_429 {
                    return Err(FetchError::Fatal(format!(
                        "{MAX_429} consecutive 429 responses, last from {url}"
                    )));
                }
                let wait = retry_after(&resp).unwrap_or(backoff_429(self.consecutive_429));
                eprintln!("  {status} from {url}; waiting {}s", wait.as_secs());
                self.sleeper.sleep(wait);
                continue;
            }
            if status == StatusCode::SERVICE_UNAVAILABLE {
                transient_attempts += 1;
                if transient_attempts >= MAX_TRANSIENT_ATTEMPTS {
                    return Err(FetchError::Transient(format!("{status} from {url}")));
                }
                let wait = retry_after(&resp).unwrap_or(backoff);
                backoff *= 2;
                eprintln!("  {status} from {url}; waiting {}s", wait.as_secs());
                self.sleeper.sleep(wait);
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
                self.sleeper.sleep(backoff);
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

// 60 s, 120 s, 240 s, 480 s for the first four 429s in a row; the fifth is fatal.
fn backoff_429(consecutive: u32) -> Duration {
    BACKOFF_429_START * 2u32.pow(consecutive.saturating_sub(1).min(3))
}

fn redirect_target(from: &str, resp: &Response) -> Option<String> {
    let location = resp.headers().get("location")?.to_str().ok()?;
    let base = Url::parse(from).ok()?;
    Some(base.join(location).ok()?.to_string())
}

fn retry_after(resp: &Response) -> Option<Duration> {
    let v = resp.headers().get("retry-after")?.to_str().ok()?;
    retry_after_value(v.trim(), time::OffsetDateTime::now_utc())
}

const HTTP_DATE: &[time::format_description::BorrowedFormatItem<'static>] = time::macros::format_description!(
    "[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT"
);

fn retry_after_value(v: &str, now: time::OffsetDateTime) -> Option<Duration> {
    if let Ok(secs) = v.parse::<u64>() {
        return Some(Duration::from_secs(secs.max(1)));
    }
    let at = time::PrimitiveDateTime::parse(v, HTTP_DATE)
        .ok()?
        .assume_utc();
    let secs = (at - now).whole_seconds().max(1) as u64;
    Some(Duration::from_secs(secs))
}

#[cfg(test)]
pub mod testing {
    use super::Sleeper;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    pub struct Request {
        pub line: String,
        pub body: String,
    }

    // Serves one canned response per connection and hands back each request.
    pub fn serve(responses: Vec<String>) -> (String, std::thread::JoinHandle<Vec<Request>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for body in responses {
                let (mut stream, _) = listener.accept().unwrap();
                seen.push(read_request(&mut stream));
                stream.write_all(body.as_bytes()).unwrap();
            }
            seen
        });
        (format!("http://{addr}"), handle)
    }

    fn read_request(stream: &mut std::net::TcpStream) -> Request {
        let mut raw = Vec::new();
        let mut buf = [0u8; 4096];
        let header_end = loop {
            if let Some(i) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                break i + 4;
            }
            let n = stream.read(&mut buf).unwrap();
            if n == 0 {
                break raw.len();
            }
            raw.extend_from_slice(&buf[..n]);
        };
        let head = String::from_utf8_lossy(&raw[..header_end]).to_string();
        let length: usize = head
            .lines()
            .find_map(|l| {
                l.strip_prefix("content-length: ")
                    .or(l.strip_prefix("Content-Length: "))
            })
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);
        while raw.len() < header_end + length {
            let n = stream.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            raw.extend_from_slice(&buf[..n]);
        }
        Request {
            line: head.lines().next().unwrap_or("").to_string(),
            body: String::from_utf8_lossy(&raw[header_end..]).to_string(),
        }
    }

    pub fn response(status: &str, headers: &[(&str, &str)], body: &str) -> String {
        let mut out = format!("HTTP/1.1 {status}\r\n");
        for (k, v) in headers {
            out.push_str(&format!("{k}: {v}\r\n"));
        }
        out.push_str(&format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ));
        out
    }

    #[derive(Clone, Default)]
    pub struct Recorder(pub Arc<Mutex<Vec<Duration>>>);

    impl Recorder {
        pub fn slept(&self) -> Vec<Duration> {
            self.0.lock().unwrap().clone()
        }
    }

    impl Sleeper for Recorder {
        fn sleep(&mut self, d: Duration) {
            self.0.lock().unwrap().push(d);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{response, serve, Recorder};
    use super::*;

    #[test]
    fn retry_after_accepts_seconds_and_http_dates() {
        let now = time::macros::datetime!(1994-11-06 08:49:00 UTC);
        assert_eq!(
            retry_after_value("120", now),
            Some(Duration::from_secs(120))
        );
        assert_eq!(retry_after_value("0", now), Some(Duration::from_secs(1)));
        assert_eq!(
            retry_after_value("Sun, 06 Nov 1994 08:49:37 GMT", now),
            Some(Duration::from_secs(37))
        );
        assert_eq!(
            retry_after_value("Sun, 06 Nov 1994 08:00:00 GMT", now),
            Some(Duration::from_secs(1))
        );
        assert_eq!(retry_after_value("soon", now), None);
    }

    #[test]
    fn redirects_are_followed_through_the_paced_loop() {
        let (base, handle) = serve(vec![
            response("302 Found", &[("Location", "/thumb/x.jpg")], ""),
            response("200 OK", &[("Content-Type", "image/jpeg")], "abc"),
        ]);
        let mut client = Client::new().unwrap();
        let started = Instant::now();
        let resp = client
            .get(&format!("{base}/wiki/Special:FilePath/x.jpg"), None)
            .unwrap();
        assert_eq!(resp.bytes().unwrap().as_ref(), b"abc");
        assert_eq!(client.requests, 2);
        assert!(
            started.elapsed() >= Duration::from_secs(1),
            "second hop was not paced"
        );
        let seen = handle.join().unwrap();
        assert_eq!(seen[0].line, "GET /wiki/Special:FilePath/x.jpg HTTP/1.1");
        assert_eq!(seen[1].line, "GET /thumb/x.jpg HTTP/1.1");
    }

    #[test]
    fn a_post_carries_its_form_body_through_the_paced_loop() {
        let (base, handle) = serve(vec![response("200 OK", &[], "ok")]);
        let mut client = Client::new().unwrap();
        let resp = client
            .send(&format!("{base}/api/wikidata"), |c, u| {
                c.post(u).form(&[("query", "SELECT ?p WHERE { ?p ?q ?r }")])
            })
            .unwrap();
        assert_eq!(resp.text().unwrap(), "ok");
        assert_eq!(client.requests, 1);
        let seen = handle.join().unwrap();
        assert_eq!(seen[0].line, "POST /api/wikidata HTTP/1.1");
        assert_eq!(
            seen[0].body,
            "query=SELECT+%3Fp+WHERE+%7B+%3Fp+%3Fq+%3Fr+%7D"
        );
    }

    #[test]
    fn requests_are_counted_per_host_family() {
        let mut client = Client::new().unwrap();
        for url in [
            "https://qlever.dev/api/wikidata",
            "https://qlever.dev/api/wikidata",
            "https://commons.wikimedia.org/w/api.php",
            "https://upload.wikimedia.org/x.jpg",
            "https://en.wikipedia.org/wiki/x",
            "https://api.cloudflare.com/client/v4",
        ] {
            client.pacer_for(url).requests += 1;
        }
        assert_eq!(client.qlever_requests(), 2);
        assert_eq!(client.wikimedia_requests(), 3);
        assert_eq!(client.requests, 0);
    }

    #[test]
    fn qlever_is_paced_three_seconds_apart_and_wikimedia_one_and_a_half() {
        assert_eq!(QLEVER_GAP, Duration::from_secs(3));
        assert_eq!(WIKIMEDIA_GAP, Duration::from_millis(1500));
    }

    #[test]
    fn five_429s_in_a_row_are_fatal_after_a_doubling_minute_ladder() {
        let too_many = response("429 Too Many Requests", &[], "");
        let (base, handle) = serve(vec![too_many; 5]);
        let recorder = Recorder::default();
        let mut client = Client::with_sleeper(Box::new(recorder.clone())).unwrap();
        let err = client.get(&format!("{base}/x"), None).unwrap_err();
        assert!(
            matches!(&err, FetchError::Fatal(m) if m.contains("5 consecutive 429")),
            "{err}"
        );
        assert_eq!(client.requests, 5);
        assert_eq!(
            recorder.slept(),
            [60, 120, 240, 480].map(Duration::from_secs)
        );
        assert_eq!(handle.join().unwrap().len(), 5);
    }

    #[test]
    fn a_429_with_retry_after_waits_that_long_and_a_success_resets_the_count() {
        let (base, handle) = serve(vec![
            response("429 Too Many Requests", &[("Retry-After", "9")], ""),
            response("200 OK", &[], "ok"),
            response("429 Too Many Requests", &[], ""),
            response("200 OK", &[], "ok"),
        ]);
        let recorder = Recorder::default();
        let mut client = Client::with_sleeper(Box::new(recorder.clone())).unwrap();
        assert_eq!(
            client
                .get(&format!("{base}/x"), None)
                .unwrap()
                .text()
                .unwrap(),
            "ok"
        );
        assert_eq!(
            client
                .get(&format!("{base}/y"), None)
                .unwrap()
                .text()
                .unwrap(),
            "ok"
        );
        assert_eq!(client.requests, 4);
        assert_eq!(
            recorder.slept(),
            [Duration::from_secs(9), Duration::from_secs(60)]
        );
        assert_eq!(handle.join().unwrap().len(), 4);
    }

    #[test]
    fn a_503_waits_for_retry_after_then_succeeds() {
        let (base, handle) = serve(vec![
            response("503 Service Unavailable", &[("Retry-After", "7")], ""),
            response("200 OK", &[], "ok"),
        ]);
        let recorder = Recorder::default();
        let mut client = Client::with_sleeper(Box::new(recorder.clone())).unwrap();
        let resp = client.get(&format!("{base}/x"), None).unwrap();
        assert_eq!(resp.text().unwrap(), "ok");
        assert_eq!(client.requests, 2);
        assert_eq!(recorder.slept(), [Duration::from_secs(7)]);
        assert_eq!(handle.join().unwrap().len(), 2);
    }

    #[test]
    fn a_500_backs_off_twice_then_gives_up_as_transient() {
        let boom = response("500 Internal Server Error", &[], "");
        let (base, handle) = serve(vec![boom.clone(), boom.clone(), boom]);
        let recorder = Recorder::default();
        let mut client = Client::with_sleeper(Box::new(recorder.clone())).unwrap();
        let err = client.get(&format!("{base}/x"), None).unwrap_err();
        assert!(matches!(err, FetchError::Transient(_)), "{err}");
        assert_eq!(client.requests, 3);
        assert_eq!(
            recorder.slept(),
            [Duration::from_secs(30), Duration::from_secs(60)]
        );
        assert_eq!(handle.join().unwrap().len(), 3);
    }
}
