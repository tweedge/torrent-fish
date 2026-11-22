// Integration test: run the real binary against a real, vendored torrent and
// verify that (1) torrent metadata is served to a magnet-only peer, and (2) no
// torrent *data* is downloaded. Both the serving peer and the metadata-fetching
// peer are this same binary, so the test has no external dependencies.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// A real, freely-distributable torrent (Big Buck Bunny, CC-BY) vendored as a
/// fixture so the test does not synthesize its own.
const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/big-buck-bunny.torrent"
);

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_torrent-fish"))
}

fn tempdir(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "tf-it-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[derive(Default, Debug)]
struct Metrics {
    total_payload_download: i64,
    total_wanted: i64,
    has_metadata: bool,
    upload_mode: bool,
    auto_managed: bool,
    paused: bool,
}

fn read_metrics(child: &mut Child) -> Option<Metrics> {
    let stdout = child.stdout.take()?;
    let reader = BufReader::new(stdout);
    let mut m = Metrics::default();
    for line in reader.lines().map_while(Result::ok) {
        if let Some(rest) = line.strip_prefix("METRIC ") {
            let (k, v) = rest.split_once('=')?;
            match k {
                "total_payload_download" => m.total_payload_download = v.parse().ok()?,
                "total_wanted" => m.total_wanted = v.parse().ok()?,
                "has_metadata" => m.has_metadata = v == "1",
                "upload_mode" => m.upload_mode = v == "1",
                "auto_managed" => m.auto_managed = v == "1",
                "paused" => m.paused = v == "1",
                _ => {}
            }
        }
        if line == "METRICS_END" {
            return Some(m);
        }
    }
    None
}

/// Use the same binary as a magnet-only peer to fetch metadata from the fish at
/// `port`. Returns the torrent name on success.
fn probe_metadata(torrent: &Path, port: u16) -> Option<String> {
    let out = Command::new(bin())
        .arg("--probe-metadata")
        .arg(torrent)
        .arg(port.to_string())
        .arg("20000")
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout
        .lines()
        .find_map(|l| l.strip_prefix("PROBE_OK ").map(|s| s.to_string()))
        .filter(|_| out.status.success())
}

#[test]
fn serves_metadata_and_downloads_no_data() {
    let fixture = Path::new(FIXTURE);
    assert!(
        fixture.exists(),
        "missing vendored torrent fixture: {FIXTURE}"
    );

    let dir = tempdir("serve");
    let torrents = dir.join("torrents");
    std::fs::create_dir_all(&torrents).unwrap();
    let staged = torrents.join("big-buck-bunny.torrent");
    std::fs::copy(fixture, &staged).unwrap();
    let save = dir.join("save");

    let port = 17801u16;
    let mut child = Command::new(bin())
        .arg("-l")
        .arg(&torrents)
        .arg("-s")
        .arg(&save)
        .arg("-p")
        .arg(port.to_string())
        .env("TF_METRICS", "1")
        .env("TF_METRICS_SECS", "6")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn torrent-fish");

    // Give the session a moment to add the torrent and start listening.
    std::thread::sleep(Duration::from_millis(800));

    let served_name = probe_metadata(&staged, port);
    let metrics = read_metrics(&mut child).expect("metrics output");
    let _ = child.wait();

    assert_eq!(
        served_name.as_deref(),
        Some("Big Buck Bunny"),
        "magnet peer did not receive metadata for the torrent from torrent-fish"
    );
    assert!(metrics.has_metadata, "fish torrent lacks metadata");
    assert!(metrics.upload_mode, "upload_mode must be set");
    assert!(!metrics.auto_managed, "auto_managed must be disabled");
    assert!(!metrics.paused, "torrent must be resumed (not paused)");
    assert_eq!(metrics.total_wanted, 0, "fish must want no pieces");

    // Decisive proof: zero payload bytes. `total_download` includes protocol
    // overhead (metadata exchange), but payload is actual torrent data.
    assert_eq!(
        metrics.total_payload_download, 0,
        "fish downloaded torrent payload data ({} bytes)",
        metrics.total_payload_download
    );

    // And nothing was written to the save path.
    if save.exists() {
        let entries: Vec<_> = std::fs::read_dir(&save).unwrap().flatten().collect();
        assert!(entries.is_empty(), "fish wrote data to save path");
    }
}
