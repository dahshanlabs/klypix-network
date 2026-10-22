//! `klypix-network signin`: this machine's name, verified with a KLYPIX account.
//!
//! The browser goes to [`CONNECT_URL`] with this machine's endpoint id, a one-time `state` and the port of a listener
//! on 127.0.0.1. Once the person signs in and says Connect, KLYPIX signs a proof for that endpoint and the page sends
//! the browser back to `http://127.0.0.1:<port>/callback?state=…&proof=…`. Where no browser can reach this machine
//! (over SSH, say) the page shows the proof as a code, and pasting it here does the same.
//!
//! A proof is checked here before anything uses it: signed with KLYPIX's key and naming this machine. So whatever
//! reaches the listener or the terminal, the worst it can do is fail; the state only keeps stray requests out.

use anyhow::{Result, anyhow, bail};
use data_encoding::HEXLOWER;
use iroh::EndpointId;
use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

pub const CONNECT_URL: &str = "https://klypix.com/network/connect";
/// How long the browser has to come back.
const WAIT: Duration = Duration::from_secs(15 * 60);
/// A request line and headers longer than this are refused (a proof is about 2 KB).
const MAX_REQUEST: usize = 16 << 10;

/// The page that signs a proof for `endpoint`; with a port and state it sends the browser back here, without them it
/// only shows the code.
pub fn connect_url(endpoint: &EndpointId, back: Option<(u16, &str)>) -> String {
    match back {
        Some((port, state)) => format!("{CONNECT_URL}?endpoint={endpoint}&port={port}&state={state}"),
        None => format!("{CONNECT_URL}?endpoint={endpoint}"),
    }
}

/// Signs this machine in: the browser (unless `browser` is false) or a pasted code. Returns the checked proof.
pub async fn sign_in(endpoint: EndpointId, browser: bool) -> Result<(String, crate::proof::Proof)> {
    let mut raw = [0u8; 16];
    getrandom::fill(&mut raw).expect("system randomness");
    let state = HEXLOWER.encode(&raw);
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    let url = connect_url(&endpoint, Some((port, &state)));
    let manual = connect_url(&endpoint, None);

    let id = endpoint.to_string();
    println!("This computer: {}…{} (the sign-in page shows the same).", &id[..8], &id[id.len() - 4..]);
    if browser && open(&url) {
        println!("Opening your browser to sign in with your KLYPIX account...");
        println!("If it doesn't open, go to:\n  {url}");
    } else {
        println!("Sign in with your KLYPIX account here:\n  {url}");
    }
    println!("No browser on this computer? Open this on any device, sign in, and paste the code it shows:\n  {manual}");
    println!("Waiting (paste the code and press Enter, or Ctrl+C to stop)...");

    let pasted = async {
        let mut line = String::new();
        let mut stdin = BufReader::new(tokio::io::stdin());
        loop {
            line.clear();
            if stdin.read_line(&mut line).await? == 0 {
                // No terminal to paste into: only the browser can answer.
                return std::future::pending().await;
            }
            if line.trim().is_empty() {
                continue;
            }
            match check(line.trim(), &endpoint) {
                Ok(done) => return Ok::<_, anyhow::Error>(done),
                Err(e) => println!("{e:#}. Paste the code again, or finish in the browser."),
            }
        }
    };
    let returned = async {
        loop {
            let (stream, _) = listener.accept().await?;
            if let Some(answer) = callback(stream, &state, &endpoint).await {
                return answer;
            }
        }
    };
    tokio::select! {
        done = returned => done,
        done = pasted => done,
        _ = tokio::time::sleep(WAIT) => bail!("no sign-in after {} minutes; run `signin` again", WAIT.as_secs() / 60),
    }
}

/// Where a proof waits when sign-in comes before a team: `init` and `join` use it, then forget it.
fn kept_path() -> std::path::PathBuf {
    crate::paths::home().join("signin-proof")
}

/// Keeps a checked proof for the team this machine starts or joins next.
pub fn keep(jws: &str) -> Result<()> {
    std::fs::write(kept_path(), jws)?;
    Ok(())
}

/// The kept proof, if it still holds for this machine.
pub fn kept() -> Option<String> {
    let jws = std::fs::read_to_string(kept_path()).ok()?;
    let endpoint = crate::daemon::load_key().ok()?.public();
    crate::proof::verify(jws.trim(), &endpoint).ok().map(|_| jws.trim().to_string())
}

pub fn forget() {
    let _ = std::fs::remove_file(kept_path());
}

fn check(jws: &str, endpoint: &EndpointId) -> Result<(String, crate::proof::Proof)> {
    let proof = crate::proof::verify(jws, endpoint)?;
    Ok((jws.to_string(), proof))
}

/// Handles one request to the listener. `None` when it wasn't the callback (a favicon, a stray request, a wrong
/// state), so the next one is awaited.
async fn callback(
    mut stream: TcpStream,
    state: &str,
    endpoint: &EndpointId,
) -> Option<Result<(String, crate::proof::Proof)>> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
        let n = tokio::time::timeout(Duration::from_secs(10), stream.read(&mut chunk)).await.ok()?.ok()?;
        if n == 0 || buf.len() + n > MAX_REQUEST {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let head = String::from_utf8_lossy(&buf);
    let target = head.lines().next()?.strip_prefix("GET ")?.split(' ').next()?.to_string();
    let Some(query) = target.strip_prefix("/callback?") else {
        reply(&mut stream, "404 Not Found", "Nothing here.").await;
        return None;
    };
    let field =
        |name: &str| query.split('&').find_map(|kv| kv.strip_prefix(name)?.strip_prefix('=')).map(percent_decode);
    if field("state").as_deref() != Some(state) {
        reply(
            &mut stream,
            "400 Bad Request",
            "This sign-in link is from another attempt. Use the newest one from your terminal.",
        )
        .await;
        return None;
    }
    if field("error").is_some() {
        reply(&mut stream, "200 OK", "Sign-in cancelled. You can close this tab.").await;
        return Some(Err(anyhow!("sign-in cancelled in the browser")));
    }
    let proof = field("proof").unwrap_or_default();
    match check(&proof, endpoint) {
        Ok(done) => {
            reply(&mut stream, "200 OK", "Signed in. You can close this tab and go back to your terminal.").await;
            Some(Ok(done))
        }
        Err(e) => {
            reply(&mut stream, "400 Bad Request", "That sign-in didn't check out. Run signin again in your terminal.")
                .await;
            Some(Err(e))
        }
    }
}

async fn reply(stream: &mut TcpStream, status: &str, text: &str) {
    let body = format!(
        "<!doctype html><meta charset=utf-8><meta name=viewport content=\"width=device-width\"><title>KLYPIX network</title>\
         <body style=\"font:16px system-ui,sans-serif;max-width:32rem;margin:15vh auto;padding:0 1rem\"><h1 style=\"font-size:1.2rem\">KLYPIX network</h1><p>{text}</p>"
    );
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes()).await;
    let _ = stream.write_all(body.as_bytes()).await;
    let _ = stream.shutdown().await;
}

/// `%XX` and `+` decoded; anything malformed is kept as it is (the proof check then refuses it).
fn percent_decode(s: &str) -> String {
    let hex = |c: u8| (c as char).to_digit(16).map(|d| d as u8);
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match (b[i], b.get(i + 1).copied().and_then(hex), b.get(i + 2).copied().and_then(hex)) {
            (b'%', Some(hi), Some(lo)) => {
                out.push(hi << 4 | lo);
                i += 3;
            }
            (b'+', ..) => {
                out.push(b' ');
                i += 1;
            }
            (c, ..) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Opens `url` in the default browser; false when that couldn't even start.
fn open(url: &str) -> bool {
    use std::process::{Command, Stdio};
    let mut cmd = if cfg!(windows) {
        // Not `cmd /c start`: cmd reads the `&` in the query as a command separator.
        let mut c = Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", url]);
        c
    } else if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(url);
        c
    } else {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::SecretKey;

    fn endpoint() -> EndpointId {
        SecretKey::from_bytes(&[7u8; 32]).public()
    }

    #[test]
    fn the_connect_url_carries_only_the_endpoint_port_and_state() {
        let e = endpoint();
        assert_eq!(connect_url(&e, Some((4242, "ab12"))), format!("{CONNECT_URL}?endpoint={e}&port=4242&state=ab12"));
        assert_eq!(connect_url(&e, None), format!("{CONNECT_URL}?endpoint={e}"));
        assert!(
            e.to_string().chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
                && e.to_string().len() == 64
        );
    }

    #[test]
    fn percent_decoding_keeps_what_it_cannot_read() {
        assert_eq!(percent_decode("a.b_c-d"), "a.b_c-d");
        assert_eq!(percent_decode("a%2Eb%2e"), "a.b.");
        assert_eq!(percent_decode("x+y"), "x y");
        assert_eq!(percent_decode("bad%zz"), "bad%zz");
        assert_eq!(percent_decode("end%2"), "end%2");
        assert_eq!(percent_decode("%"), "%");
    }

    async fn get(port: u16, path: &str) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        s.write_all(format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").as_bytes()).await.unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).await.unwrap();
        out
    }

    #[tokio::test]
    async fn the_callback_takes_only_the_right_state_and_a_real_proof() {
        let e = endpoint();
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let serve = |listener: TcpListener| async move {
            let (s, _) = listener.accept().await.unwrap();
            (callback(s, "good", &endpoint()).await, listener)
        };

        let (task, client) = (tokio::spawn(serve(listener)), get(port, "/favicon.ico"));
        let page = client.await;
        let (answer, listener) = task.await.unwrap();
        assert!(answer.is_none() && page.starts_with("HTTP/1.1 404"), "a stray request is not the callback");

        let (task, client) = (tokio::spawn(serve(listener)), get(port, "/callback?state=bad&proof=x"));
        let page = client.await;
        let (answer, listener) = task.await.unwrap();
        assert!(answer.is_none() && page.starts_with("HTTP/1.1 400"), "another attempt's state is refused");

        let (task, client) = (tokio::spawn(serve(listener)), get(port, "/callback?state=good&proof=not.a.proof"));
        let page = client.await;
        let (answer, listener) = task.await.unwrap();
        assert!(
            matches!(answer, Some(Err(_))) && page.starts_with("HTTP/1.1 400"),
            "a proof that doesn't verify fails"
        );

        let (task, client) = (tokio::spawn(serve(listener)), get(port, "/callback?state=good&error=cancelled"));
        let page = client.await;
        let (answer, _) = task.await.unwrap();
        let err = answer.unwrap().unwrap_err().to_string();
        assert!(err.contains("cancelled") && page.contains("cancelled"), "{err}");
        assert_eq!(e, endpoint());
    }

    #[test]
    fn a_proof_for_this_machine_signed_by_klypix_is_taken() {
        let e = endpoint();
        let jws = crate::proof::tests::proof("user_1", "alex", &e);
        let (got, proof) = check(&jws, &e).expect("a good proof");
        assert_eq!(got, jws);
        assert_eq!(proof.endpoint, e.to_string());
        let other = SecretKey::from_bytes(&[8u8; 32]).public();
        assert!(check(&jws, &other).is_err(), "a proof for another machine is refused");
    }
}
