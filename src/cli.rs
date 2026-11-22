// CLI parsing and option normalization, mirroring torrent-fish.py's optparse
// behavior: same flags, defaults, conversions, and validation.

use std::ffi::OsString;

pub const DEFAULT_PORT: u16 = 6881;
pub const DEFAULT_PROXY_PORT: u16 = 8080;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proxy {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub port: u16,
    pub listen_interface: String,
    pub outgoing_interface: String,
    pub max_download_rate: i64,
    pub max_upload_rate: i64,
    pub connections_limit: i64,
    pub load_path: String,
    pub save_path: String,
    pub proxy: Option<Proxy>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            port: DEFAULT_PORT,
            listen_interface: "0.0.0.0".to_string(),
            outgoing_interface: String::new(),
            max_download_rate: -1,
            max_upload_rate: -1,
            connections_limit: 800,
            load_path: "/opt".to_string(),
            save_path: "/opt".to_string(),
            proxy: None,
        }
    }
}

pub enum ParseResult {
    Run(Options),
    Help(String),
}

/// Convert a raw port into a usable TCP port, falling back to the default when
/// out of range (matches the Python `sanitize_port`).
pub fn sanitize_port(port: i64) -> u16 {
    if (1..=65535).contains(&port) {
        port as u16
    } else {
        DEFAULT_PORT
    }
}

/// Convert a rate in kB/s to libtorrent's bytes/s limit (-1 means infinite).
pub fn convert_rate_limit(rate_kbps: f64) -> i64 {
    let rate = (rate_kbps * 1000.0) as i64;
    if rate > 0 {
        rate
    } else {
        -1
    }
}

/// Parse `host[:port]` into a proxy, tolerating missing/invalid ports by
/// falling back to the default (matches the hardened Python behavior).
pub fn parse_proxy_host(value: &str) -> Option<Proxy> {
    if value.is_empty() {
        return None;
    }
    let (host, port) = match value.split_once(':') {
        Some((h, p)) => (h, p.parse::<i64>().unwrap_or(i64::from(DEFAULT_PROXY_PORT))),
        None => (value, i64::from(DEFAULT_PROXY_PORT)),
    };
    let port = if (1..=65535).contains(&port) {
        port as u16
    } else {
        DEFAULT_PROXY_PORT
    };
    Some(Proxy {
        host: host.to_string(),
        port,
    })
}

fn help_text() -> String {
    "\
Usage: torrent-fish [options]

Options:
  -h, --help            show this help message and exit
  -p PORT, --port=PORT  set listening port
  -i LISTEN_INTERFACE, --listen-interface=LISTEN_INTERFACE
                        set interface for incoming connections
  -o OUTGOING_INTERFACE, --outgoing-interface=OUTGOING_INTERFACE
                        set interface for outgoing connections
  -d MAX_DOWNLOAD_RATE, --max-download-rate=MAX_DOWNLOAD_RATE
                        the maximum download rate given in kB/s. 0 means
                        infinite.
  -u MAX_UPLOAD_RATE, --max-upload-rate=MAX_UPLOAD_RATE
                        the maximum upload rate given in kB/s. 0 means
                        infinite.
  -c CONNECTIONS_LIMIT, --connections_limit=CONNECTIONS_LIMIT
                        the global limit on the number of connections opened.
  -l LOAD_PATH, --load-path=LOAD_PATH
                        the path to load torrent files from.
  -s SAVE_PATH, --save-path=SAVE_PATH
                        must be a valid folder BUT won't be used to download
                        anything.
  -r PROXY_HOST, --proxy-host=PROXY_HOST
                        sets HTTP proxy host and port (separated by ':')
"
    .to_string()
}

/// Parse CLI args. Accepts both `--flag value` and `--flag=value`, plus the
/// short forms from the Python CLI.
pub fn parse<I>(args: I) -> Result<ParseResult, String>
where
    I: IntoIterator<Item = OsString>,
{
    let mut opts = Options::default();
    let mut args = args
        .into_iter()
        .map(|a| a.to_string_lossy().into_owned())
        .peekable();

    while let Some(arg) = args.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with('-') => (f.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };

        let mut take = |name: &str| -> Result<String, String> {
            if let Some(v) = inline.clone() {
                Ok(v)
            } else {
                args.next()
                    .ok_or_else(|| format!("option {name} requires an argument"))
            }
        };

        match flag.as_str() {
            "-h" | "--help" => return Ok(ParseResult::Help(help_text())),
            "-p" | "--port" => {
                let v = take("--port")?;
                let n: i64 = v.parse().map_err(|_| format!("invalid port: {v}"))?;
                opts.port = sanitize_port(n);
            }
            "-i" | "--listen-interface" => opts.listen_interface = take("--listen-interface")?,
            "-o" | "--outgoing-interface" => {
                opts.outgoing_interface = take("--outgoing-interface")?
            }
            "-d" | "--max-download-rate" => {
                let v = take("--max-download-rate")?;
                let n: f64 = v.parse().map_err(|_| format!("invalid rate: {v}"))?;
                opts.max_download_rate = convert_rate_limit(n);
            }
            "-u" | "--max-upload-rate" => {
                let v = take("--max-upload-rate")?;
                let n: f64 = v.parse().map_err(|_| format!("invalid rate: {v}"))?;
                opts.max_upload_rate = convert_rate_limit(n);
            }
            "-c" | "--connections_limit" | "--connections-limit" => {
                let v = take("--connections_limit")?;
                opts.connections_limit = v
                    .parse()
                    .map_err(|_| format!("invalid connection limit: {v}"))?;
            }
            "-l" | "--load-path" => opts.load_path = take("--load-path")?,
            "-s" | "--save-path" => opts.save_path = take("--save-path")?,
            "-r" | "--proxy-host" => {
                let v = take("--proxy-host")?;
                opts.proxy = parse_proxy_host(&v);
            }
            other => return Err(format!("no such option: {other}")),
        }
    }

    Ok(ParseResult::Run(opts))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<OsString> {
        items.iter().map(OsString::from).collect()
    }

    fn run(items: &[&str]) -> Options {
        match parse(args(items)).unwrap() {
            ParseResult::Run(o) => o,
            ParseResult::Help(_) => panic!("unexpected help"),
        }
    }

    #[test]
    fn defaults_match_python() {
        let o = run(&[]);
        assert_eq!(o.port, 6881);
        assert_eq!(o.listen_interface, "0.0.0.0");
        assert_eq!(o.outgoing_interface, "");
        assert_eq!(o.max_download_rate, -1);
        assert_eq!(o.max_upload_rate, -1);
        assert_eq!(o.connections_limit, 800);
        assert_eq!(o.load_path, "/opt");
        assert_eq!(o.save_path, "/opt");
        assert_eq!(o.proxy, None);
    }

    #[test]
    fn help_flag() {
        assert!(matches!(parse(args(&["--help"])), Ok(ParseResult::Help(_))));
        assert!(matches!(parse(args(&["-h"])), Ok(ParseResult::Help(_))));
    }

    #[test]
    fn long_flags_with_space_and_equals() {
        assert_eq!(run(&["--port", "1234"]).port, 1234);
        assert_eq!(run(&["--port=4321"]).port, 4321);
        assert_eq!(run(&["-p", "9999"]).port, 9999);
    }

    #[test]
    fn load_and_save_paths() {
        let o = run(&["-l", "/torrents", "-s", "/save"]);
        assert_eq!(o.load_path, "/torrents");
        assert_eq!(o.save_path, "/save");
    }

    #[test]
    fn rate_conversion() {
        assert_eq!(run(&["-d", "100"]).max_download_rate, 100_000);
        assert_eq!(run(&["-u", "0"]).max_upload_rate, -1);
        assert_eq!(run(&["-d", "-5"]).max_download_rate, -1);
    }

    #[test]
    fn port_sanitization_via_cli() {
        assert_eq!(run(&["-p", "0"]).port, 6881);
        assert_eq!(run(&["-p", "65536"]).port, 6881);
        assert_eq!(run(&["-p", "65535"]).port, 65535);
    }

    #[test]
    fn unknown_option_errors() {
        assert!(parse(args(&["--bogus"])).is_err());
    }

    #[test]
    fn sanitize_port_bounds() {
        assert_eq!(sanitize_port(-1), 6881);
        assert_eq!(sanitize_port(0), 6881);
        assert_eq!(sanitize_port(1), 1);
        assert_eq!(sanitize_port(65535), 65535);
        assert_eq!(sanitize_port(65536), 6881);
    }

    #[test]
    fn convert_rate_limit_bounds() {
        assert_eq!(convert_rate_limit(0.0), -1);
        assert_eq!(convert_rate_limit(-5.0), -1);
        assert_eq!(convert_rate_limit(1.0), 1000);
        assert_eq!(convert_rate_limit(2.5), 2500);
    }

    #[test]
    fn proxy_parsing() {
        assert_eq!(parse_proxy_host(""), None);
        assert_eq!(
            parse_proxy_host("1.2.3.4:8080"),
            Some(Proxy {
                host: "1.2.3.4".into(),
                port: 8080
            })
        );
        assert_eq!(
            parse_proxy_host("badproxy"),
            Some(Proxy {
                host: "badproxy".into(),
                port: 8080
            })
        );
        assert_eq!(
            parse_proxy_host("host:notaport"),
            Some(Proxy {
                host: "host".into(),
                port: 8080
            })
        );
        assert_eq!(
            parse_proxy_host("1.2.3.4:99999"),
            Some(Proxy {
                host: "1.2.3.4".into(),
                port: 8080
            })
        );
    }
}
