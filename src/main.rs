// torrent-fish: a metadata-only BitTorrent peer.
//
// Rust port of torrent-fish.py. The libtorrent interactions live in a small CXX
// bridge (`shim/torrent_fish_shim.cpp`); all policy (CLI parsing, file
// discovery, port/rate conversion, proxy parsing, skip-on-error) is in `cli` so
// it can be unit-tested without a live session.

use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

mod bridge;
mod cli;

use bridge::ffi;
use cli::{Options, ParseResult};

/// Recursively collect every regular file under `directory`, absolute. Mirrors
/// Python's `os.walk`-based helper (does not follow directory symlinks).
fn find_files(directory: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(base) = std::fs::canonicalize(directory) {
        walk(&base, &mut out);
    }
    out
}

fn walk(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        // Mirror os.walk(followlinks=False): do not recurse into symlinked
        // directories, but do include symlinked files (and anything that
        // resolves to a regular file).
        match entry.file_type() {
            Ok(ft) if ft.is_dir() => walk(&path, out),
            Ok(ft) if ft.is_file() => out.push(path.to_string_lossy().into_owned()),
            Ok(ft) if ft.is_symlink() && path.is_file() => {
                out.push(path.to_string_lossy().into_owned());
            }
            _ => {}
        }
    }
}

/// Build the libtorrent session from normalized CLI options.
fn new_session(opts: &Options) -> cxx::UniquePtr<ffi::Session> {
    let user_agent = format!(
        "libtorrent/{} (https://torrent.fish/)",
        ffi::libtorrent_version()
    );
    let listen_interfaces = format!("{}:{}", opts.listen_interface, opts.port);
    let (has_proxy, proxy_hostname, proxy_port) = match &opts.proxy {
        Some(p) => (true, p.host.as_str(), p.port),
        None => (false, "", 0),
    };
    ffi::new_session(
        &user_agent,
        &listen_interfaces,
        opts.max_download_rate,
        opts.max_upload_rate,
        opts.connections_limit,
        &opts.outgoing_interface,
        has_proxy,
        proxy_hostname,
        proxy_port,
    )
}

fn main() -> ExitCode {
    let mut args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();

    // Hidden test/probe helper: fetch metadata for a torrent from a torrent-fish
    // peer, proving metadata serving without any external torrent client.
    // Usage: torrent-fish --probe-metadata <torrent> <port> [timeout_ms]
    if args
        .first()
        .map(|a| a == "--probe-metadata")
        .unwrap_or(false)
    {
        args.remove(0);
        let torrent = args.first().map(|a| a.to_string_lossy().into_owned());
        let port = args
            .get(1)
            .and_then(|p| p.to_str())
            .and_then(|p| p.parse().ok());
        let timeout = args
            .get(2)
            .and_then(|t| t.to_str())
            .and_then(|t| t.parse().ok())
            .unwrap_or(20_000);
        let (Some(torrent), Some(port)) = (torrent, port) else {
            eprintln!("usage: torrent-fish --probe-metadata <torrent> <port> [timeout_ms]");
            return ExitCode::from(2);
        };
        return match ffi::probe_metadata(&torrent, port, timeout) {
            Ok(name) => {
                println!("PROBE_OK {name}");
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("PROBE_FAILED {err}");
                ExitCode::from(1)
            }
        };
    }

    let opts = match cli::parse(std::env::args_os().skip(1)) {
        Ok(ParseResult::Run(opts)) => opts,
        Ok(ParseResult::Help(text)) => {
            print!("{text}");
            return ExitCode::SUCCESS;
        }
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(2);
        }
    };
    run(opts)
}

fn run(opts: Options) -> ExitCode {
    let mut session = new_session(&opts);

    let torrent_files = find_files(&opts.load_path);
    if torrent_files.is_empty() {
        println!("[torrent-fish] No files found. Quitting ...");
        return ExitCode::from(1);
    }

    let mut added = 0usize;
    for torrent_file in &torrent_files {
        println!("[torrent-fish] Adding {torrent_file}");
        match session.pin_mut().add_torrent(torrent_file, &opts.save_path) {
            Ok(_name) => added += 1,
            Err(err) => println!("[torrent-fish] Skipping {torrent_file}: {err}"),
        }
    }

    if added < 1 {
        println!("[torrent-fish] No valid torrents found. Quitting ...");
        return ExitCode::from(1);
    }

    // Test/diagnostic mode: after a soak period, print accounting and exit so
    // integration tests can assert that metadata is served while no torrent
    // data is downloaded. Never enabled in normal operation.
    if std::env::var("TF_METRICS").is_ok() {
        let secs: u64 = std::env::var("TF_METRICS_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3);
        std::thread::sleep(Duration::from_secs(secs));
        for message in session.pop_alerts() {
            println!("[libtorrent] {message}");
        }
        println!("METRIC serve={}", session.listen_port());
        println!("METRIC has_metadata={}", u8::from(session.has_metadata()));
        println!("METRIC upload_mode={}", u8::from(session.upload_mode()));
        println!("METRIC auto_managed={}", u8::from(session.auto_managed()));
        println!("METRIC paused={}", u8::from(session.paused()));
        println!("METRIC total_wanted={}", session.total_wanted());
        println!("METRIC total_download={}", session.total_download());
        println!(
            "METRIC total_payload_download={}",
            session.total_payload_download()
        );
        println!("METRIC total_upload={}", session.total_upload());
        println!("METRIC num_pieces={}", session.num_pieces());
        println!("METRICS_END");
        return ExitCode::SUCCESS;
    }

    // Drain alerts, matching the Python loop's output.
    loop {
        for message in session.pop_alerts() {
            println!("[libtorrent] {message}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_files_recursively_and_absolutely() {
        let dir = tempdir();
        std::fs::create_dir_all(dir.join("a/b")).unwrap();
        std::fs::write(dir.join("top.torrent"), b"x").unwrap();
        std::fs::write(dir.join("a/b/deep.torrent"), b"y").unwrap();

        let found = find_files(dir.to_str().unwrap());
        let mut names: Vec<_> = found
            .iter()
            .map(|p| {
                Path::new(p)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        names.sort();
        assert_eq!(names, vec!["deep.torrent", "top.torrent"]);
        assert!(found.iter().all(|p| Path::new(p).is_absolute()));
    }

    #[test]
    fn missing_directory_is_empty() {
        assert!(find_files("/definitely/does/not/exist/xyz").is_empty());
    }

    fn tempdir() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "torrent-fish-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
