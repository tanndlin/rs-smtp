//! Throughput benchmark: how many messages per second the server can hand to
//! mail clients that are reading their inbox.
//!
//! Each simulated client walks the same session a desktop client does on open:
//!
//! 1. `LOGIN`, `SELECT INBOX`
//! 2. flag resync: `UID FETCH 1:* (UID FLAGS)`
//! 3. message list: `UID FETCH 1:* (UID RFC822.SIZE FLAGS INTERNALDATE ENVELOPE BODY.PEEK[HEADER])`
//! 4. open every message: `UID FETCH <uid> (BODY.PEEK[])` then
//!    `UID STORE <uid> +FLAGS.SILENT (\Seen)`
//! 5. `LOGOUT`
//!
//! All clients start each phase together, so a phase's wall time is the time
//! the server took to serve every client through it.
//!
//! Runs against a throwaway database like the integration tests (see
//! `tests/test_util`), so it needs the same Postgres: `TEST_DATABASE_URL`,
//! falling back to `DATABASE_URL`, then the docker-compose `db` service.
//!
//! ```text
//! cargo bench -p imap --bench client_read
//! ```
//!
//! Tunables (environment variables):
//! - `IMAP_BENCH_MESSAGES` - messages seeded into INBOX (default 200)
//! - `IMAP_BENCH_CLIENTS`  - concurrent client sessions (default 4)
//! - `IMAP_BENCH_POOL`     - server Postgres pool size (default 10, as in `main`)

use std::sync::Arc;
use std::time::{Duration, Instant};

use sqlx::types::chrono::Utc;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Barrier;
use util::{Email, Mailbox};

#[path = "../tests/test_util/mod.rs"]
mod test_util;
use test_util::start_server_with_pool_size;

fn env_or(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// A plausible RFC 5322 message whose body size varies from roughly 1 to 9 KB,
/// so the read phase isn't timing one fixed-size payload.
fn message(i: usize) -> Vec<u8> {
    const LINE: &str =
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod.\r\n";
    let lines = 16 + (i * 37) % 112;
    format!(
        "Date: Wed, 23 Oct 2024 19:00:00 +0000\r\n\
         From: \"Alice Example\" <alice@example.com>\r\n\
         To: bob@example.com, carol@example.net\r\n\
         Cc: dave@example.org\r\n\
         Subject: Benchmark message {i}\r\n\
         Message-ID: <bench-{i}@example.com>\r\n\
         Content-Type: text/plain; charset=us-ascii\r\n\
         \r\n\
         {}",
        LINE.repeat(lines)
    )
    .into_bytes()
}

/// A minimal IMAP client: sends tagged commands and reads up to the tagged
/// completion, stepping over `{n}` literals so message bodies can't be
/// mistaken for protocol lines.
struct Client {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    next_tag: u32,
}

/// What came back for one command.
struct Reply {
    /// Untagged `* n FETCH` responses seen.
    fetches: usize,
    /// Total bytes read off the wire.
    bytes: usize,
}

impl Client {
    async fn connect(addr: std::net::SocketAddr) -> Self {
        let stream = TcpStream::connect(addr).await.expect("connect failed");
        stream.set_nodelay(true).unwrap();
        let (read, writer) = stream.into_split();
        let mut client = Self {
            reader: BufReader::new(read),
            writer,
            next_tag: 0,
        };

        let mut greeting = Vec::new();
        client
            .reader
            .read_until(b'\n', &mut greeting)
            .await
            .unwrap();
        assert!(greeting.starts_with(b"* OK"), "bad greeting: {greeting:?}");
        client
    }

    async fn command(&mut self, cmd: &str) -> Reply {
        self.send(cmd.as_bytes()).await
    }

    /// Send `body` (everything after the tag, CRLF included) and read the reply.
    async fn send(&mut self, body: &[u8]) -> Reply {
        self.next_tag += 1;
        let tag = format!("b{}", self.next_tag);

        let mut out = Vec::with_capacity(tag.len() + 1 + body.len());
        out.extend_from_slice(tag.as_bytes());
        out.push(b' ');
        out.extend_from_slice(body);
        self.writer.write_all(&out).await.expect("write failed");

        self.read_reply(&tag).await
    }

    async fn read_reply(&mut self, tag: &str) -> Reply {
        let done = format!("{tag} ");
        let mut reply = Reply {
            fetches: 0,
            bytes: 0,
        };
        let mut line = Vec::new();
        let mut literal = Vec::new();

        loop {
            line.clear();
            let n = self.reader.read_until(b'\n', &mut line).await.unwrap();
            assert!(n > 0, "server closed the connection awaiting {tag}");
            reply.bytes += n;

            if line.starts_with(done.as_bytes()) {
                assert!(
                    line[done.len()..].starts_with(b"OK"),
                    "{tag} failed: {}",
                    String::from_utf8_lossy(&line)
                );
                return reply;
            }
            if line.starts_with(b"* ") && line.windows(7).any(|w| w == b" FETCH ") {
                reply.fetches += 1;
            }

            // A line ending in `{n}` is followed by n raw octets; the line then
            // carries on after them, so keep consuming until a plain CRLF.
            let mut tail = line.as_slice();
            while let Some(len) = literal_len(tail) {
                literal.resize(len, 0);
                self.reader.read_exact(&mut literal).await.unwrap();
                reply.bytes += len;

                line.clear();
                reply.bytes += self.reader.read_until(b'\n', &mut line).await.unwrap();
                tail = line.as_slice();
            }
        }
    }
}

/// The `n` of a line ending in `{n}\r\n`.
fn literal_len(line: &[u8]) -> Option<usize> {
    let line = line.strip_suffix(b"\r\n")?.strip_suffix(b"}")?;
    let open = line.iter().rposition(|&b| b == b'{')?;
    std::str::from_utf8(&line[open + 1..]).ok()?.parse().ok()
}

/// Per-client timings for each phase.
#[derive(Default)]
struct ClientStats {
    open_latencies: Vec<Duration>,
    bytes: usize,
}

async fn run_client(
    addr: std::net::SocketAddr,
    messages: usize,
    barrier: Arc<Barrier>,
) -> ClientStats {
    let mut stats = ClientStats::default();
    let mut client = Client::connect(addr).await;

    // Phase 1: login + select.
    barrier.wait().await;
    client.command("LOGIN \"admin\" \"password\"\r\n").await;
    client.command("SELECT INBOX\r\n").await;
    barrier.wait().await;

    // Phase 2: flag resync.
    barrier.wait().await;
    let r = client.command("UID FETCH 1:* (UID FLAGS)\r\n").await;
    assert_eq!(r.fetches, messages, "flag resync missed messages");
    stats.bytes += r.bytes;
    barrier.wait().await;

    // Phase 3: message list.
    barrier.wait().await;
    let r = client
        .command(
            "UID FETCH 1:* (UID RFC822.SIZE FLAGS INTERNALDATE ENVELOPE BODY.PEEK[HEADER])\r\n",
        )
        .await;
    assert_eq!(r.fetches, messages, "message list missed messages");
    stats.bytes += r.bytes;
    barrier.wait().await;

    // Phase 4: open every message.
    barrier.wait().await;
    for uid in 1..=messages {
        let start = Instant::now();
        let r = client
            .command(&format!("UID FETCH {uid} (BODY.PEEK[])\r\n"))
            .await;
        assert_eq!(r.fetches, 1, "UID {uid} not returned");
        client
            .command(&format!("UID STORE {uid} +FLAGS.SILENT (\\Seen)\r\n"))
            .await;
        stats.open_latencies.push(start.elapsed());
        stats.bytes += r.bytes;
    }
    barrier.wait().await;

    client.command("LOGOUT\r\n").await;
    stats
}

/// Time between two barrier releases, observed by the coordinator task.
async fn timed_phase(barrier: &Barrier) -> Duration {
    barrier.wait().await;
    let start = Instant::now();
    barrier.wait().await;
    start.elapsed()
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx]
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let messages = env_or("IMAP_BENCH_MESSAGES", 200);
    let clients = env_or("IMAP_BENCH_CLIENTS", 4);
    let pool = env_or("IMAP_BENCH_POOL", 10) as u32;

    let server = start_server_with_pool_size(pool).await;
    let addr = server.addr;

    // Seed the inbox the way APPEND stores a message. This goes straight to
    // the database because APPEND can't yet take a literal bigger than one
    // socket read (`Cursor::raw` assumes the whole literal is buffered).
    let seed_start = Instant::now();
    let db = server.db_pool().await;
    let mut seeded_bytes = 0;
    for i in 0..messages {
        let raw = message(i);
        seeded_bytes += raw.len();
        let email = Email::from_raw(Utc::now(), String::from_utf8(raw).unwrap());

        let mut tx = db.begin().await.unwrap();
        let (uid, _) = Mailbox::allocate_uid(&mut *tx, "INBOX").await.unwrap();
        email.insert(&mut *tx, "INBOX", uid).await.unwrap();
        tx.commit().await.unwrap();
    }
    let seed_time = seed_start.elapsed();

    println!(
        "IMAP client-read benchmark: {messages} messages ({:.1} KB avg), {clients} clients, pool {pool}",
        seeded_bytes as f64 / messages.max(1) as f64 / 1024.0
    );
    println!("  seeded in {:.0} ms\n", ms(seed_time));

    // Every client plus this coordinator meets at each phase boundary.
    let barrier = Arc::new(Barrier::new(clients + 1));
    let handles: Vec<_> = (0..clients)
        .map(|_| tokio::spawn(run_client(addr, messages, barrier.clone())))
        .collect();

    let login = timed_phase(&barrier).await;
    let resync = timed_phase(&barrier).await;
    let list = timed_phase(&barrier).await;
    let read = timed_phase(&barrier).await;

    let mut latencies = Vec::new();
    let mut bytes = 0;
    for handle in handles {
        let stats = handle.await.expect("client task panicked");
        latencies.extend(stats.open_latencies);
        bytes += stats.bytes;
    }
    latencies.sort();

    let total = (messages * clients) as f64;
    let row = |name: &str, wall: Duration, per_sec: Option<f64>| {
        let rate = per_sec.map_or(String::new(), |r| format!("{r:>10.0} msg/s"));
        println!("  {name:<28} {:>9.1} ms {rate}", ms(wall));
    };
    row("login + select", login, None);
    row(
        "flag resync (UID FLAGS)",
        resync,
        Some(total / resync.as_secs_f64()),
    );
    row(
        "message list (headers)",
        list,
        Some(total / list.as_secs_f64()),
    );
    row(
        "open messages (BODY[]+\\Seen)",
        read,
        Some(total / read.as_secs_f64()),
    );

    println!(
        "\n  open latency  p50 {:.2} ms  p95 {:.2} ms  p99 {:.2} ms  max {:.2} ms",
        ms(percentile(&latencies, 0.50)),
        ms(percentile(&latencies, 0.95)),
        ms(percentile(&latencies, 0.99)),
        ms(percentile(&latencies, 1.0)),
    );
    let session = login + resync + list + read;
    println!(
        "  full session: {total:.0} messages read in {:.0} ms -> {:.0} emails/s ({:.1} MB/s)",
        ms(session),
        total / session.as_secs_f64(),
        bytes as f64 / session.as_secs_f64() / 1_000_000.0
    );
}
