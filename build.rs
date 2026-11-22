use std::env;
use std::path::PathBuf;

fn main() {
    let mut bridge = cxx_build::bridge("src/bridge.rs");
    bridge
        .file("shim/torrent_fish_shim.cpp")
        .include("shim")
        .std("c++17");

    // Standard non-sysroot prefixes (e.g. `make install` / Alpine /docker).
    for inc in ["/usr/local/include", "/usr/include"] {
        if PathBuf::from(inc).exists() {
            bridge.include(inc);
        }
    }
    for lib in ["/usr/local/lib", "/usr/lib/x86_64-linux-gnu", "/usr/lib"] {
        if PathBuf::from(lib).exists() {
            println!("cargo:rustc-link-search=native={lib}");
        }
    }

    // Optional sysroot for a self-contained local measurement harness.
    if let Ok(sysroot) = env::var("LT_SYSROOT") {
        let root = PathBuf::from(&sysroot);
        for sub in ["usr/include", "usr/include/x86_64-linux-gnu"] {
            let inc = root.join(sub);
            if inc.exists() {
                bridge.include(inc);
            }
        }
        for sub in ["usr/lib/x86_64-linux-gnu", "usr/lib"] {
            let lib = root.join(sub);
            if lib.exists() {
                println!("cargo:rustc-link-search=native={}", lib.display());
            }
        }
    }

    bridge.compile("torrent-fish-shim");

    // A fully static binary (scratch image) links libtorrent/boost/openssl
    // statically; otherwise link dynamically to the system library.
    let static_link = env::var("TF_STATIC_LINK").is_ok();
    // Static libtorrent and boost reference each other, so the archives must be
    // wrapped in --start-group/--end-group, which `cargo:rustc-link-lib` cannot
    // express. Emit explicit link args (appended after our shim object, so the
    // group resolves the shim's libtorrent references too).
    println!("cargo:rustc-link-search=native=/usr/local/lib");
    if static_link {
        println!("cargo:rustc-link-search=native=/usr/lib");
        // One group over every static archive and the C/C++ runtimes. Using -l
        // names (not explicit paths) keeps the linker's archive resolution and
        // the group resolves the circular libtorrent<->boost<->libc refs.
        println!("cargo:rustc-link-arg=-Wl,--start-group");
        for lib in [
            "torrent-rasterbar",
            "boost_system",
            "boost_filesystem",
            "boost_chrono",
            "boost_random",
            "boost_date_time",
            "boost_thread",
            "ssl",
            "crypto",
            "z",
            "stdc++",
            "gcc",
            "gcc_eh",
            "c",
        ] {
            println!("cargo:rustc-link-arg=-l{lib}");
        }
        println!("cargo:rustc-link-arg=-Wl,--end-group");
    } else {
        println!("cargo:rustc-link-lib=torrent-rasterbar");
    }
    println!("cargo:rustc-link-lib=pthread");

    println!("cargo:rerun-if-changed=shim/torrent_fish_shim.cpp");
    println!("cargo:rerun-if-changed=shim/torrent_fish_shim.h");
    println!("cargo:rerun-if-changed=src/bridge.rs");
    println!("cargo:rerun-if-env-changed=LT_SYSROOT");
    println!("cargo:rerun-if-env-changed=TF_STATIC_LINK");
}
