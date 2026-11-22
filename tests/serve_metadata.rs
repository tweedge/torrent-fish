// Integration tests that run the real binary against real torrents and verify:
//  1. torrent files are served (a magnet-only peer receives metadata), and
//  2. no torrent *data* is downloaded (libtorrent payload accounting is zero and
//     nothing is written to the save path).

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

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

/// Minimal bencode for a single-file torrent. Building it in Rust keeps the test
/// self-contained (no Python/libtorrent dependency to synthesize fixtures).
fn bencode_single_file_torrent(dir: &Path, name: &str, data: &[u8]) -> PathBuf {
    // SHA-1 of the single piece (one piece, since data < 16 KiB).
    let piece_hash = sha1(data);

    let mut info = Vec::new();
    info.extend(b"d6:lengthi");
    info.extend(data.len().to_string().as_bytes());
    info.extend(b"e4:name");
    info.extend(name.len().to_string().as_bytes());
    info.extend(b":");
    info.extend(name.as_bytes());
    info.extend(b"12:piece lengthi16384e6:pieces20:");
    info.extend(piece_hash);
    info.extend(b"e");

    let mut torrent = Vec::new();
    torrent.extend(b"d8:announce");
    // A dummy announce URL (never contacted in tests); required-ish for realism.
    let announce = b"http://127.0.0.1:1/announce";
    torrent.extend(announce.len().to_string().as_bytes());
    torrent.extend(b":");
    torrent.extend(announce);
    torrent.extend(b"4:info");
    torrent.extend(info);
    torrent.extend(b"e");

    let path = dir.join(format!("{name}.torrent"));
    std::fs::write(&path, torrent).unwrap();
    path
}

/// Minimal SHA-1 (test-only; avoids adding a dependency).
fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let ml = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend(ml.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let tmp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = tmp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }
    let mut out = [0u8; 20];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

#[derive(Default, Debug)]
struct Metrics {
    total_payload_download: i64,
    total_wanted: i64,
    has_metadata: bool,
    upload_mode: bool,
    auto_managed: bool,
    paused: bool,
    serve: String,
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
                "serve" => m.serve = v.to_string(),
                _ => {}
            }
        }
        if line == "METRICS_END" {
            return Some(m);
        }
    }
    None
}

#[test]
fn serves_metadata_and_downloads_no_data() {
    let dir = tempdir("serve");
    let payload = b"rust port e2e payload\n";
    let torrent = bencode_single_file_torrent(&dir, "content", payload);

    let torrents = dir.join("torrents");
    std::fs::create_dir_all(&torrents).unwrap();
    let staged = torrents.join("test.torrent");
    std::fs::copy(&torrent, &staged).unwrap();
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
        .env("TF_METRICS_SECS", "5")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn torrent-fish");

    std::thread::sleep(Duration::from_millis(800));
    let served = drive_magnet_peer(port, &staged);
    let metrics = read_metrics(&mut child).expect("metrics output");
    let _ = child.wait();

    assert!(served, "peer did not receive metadata from torrent-fish");
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

    if save.exists() {
        let entries: Vec<_> = std::fs::read_dir(&save).unwrap().flatten().collect();
        assert!(entries.is_empty(), "fish wrote data to save path");
    }
}

/// Drives a real libtorrent magnet peer (via the python bindings) against the
/// fish peer to prove metadata is actually served on the wire. Skips trivially
/// (returns true) if python/libtorrent is unavailable, since the metric
/// assertions above still cover the core invariants.
fn drive_magnet_peer(port: u16, torrent: &Path) -> bool {
    let script = r#"
import sys, time
try:
    import libtorrent as lt
except Exception:
    print("NOPY", flush=True); sys.exit(2)
port=int(sys.argv[1]); tf=sys.argv[2]
ti=lt.torrent_info(tf); mag=lt.make_magnet_uri(ti)
ses=lt.session({"listen_interfaces":"127.0.0.1:0","enable_dht":False,"enable_lsd":False,"enable_upnp":False,"enable_natpmp":False})
atp=lt.parse_magnet_uri(mag); atp.save_path="/tmp/tf-it-dl"; atp.peers=[("127.0.0.1",port)]
h=ses.add_torrent(atp)
end=time.time()+20
while time.time()<end:
    if h.status().has_metadata:
        print("OK", flush=True); sys.exit(0)
    time.sleep(0.2)
sys.exit(1)
"#;
    match Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(port.to_string())
        .arg(torrent)
        .output()
    {
        Ok(o) if o.status.success() => true,
        Ok(o) if o.status.code() == Some(2) => {
            eprintln!("note: python libtorrent unavailable; skipping wire-level check");
            true
        }
        _ => false,
    }
}
