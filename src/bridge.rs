// cxx bridge over the libtorrent operations torrent-fish needs. Uses only
// primitives and rust types, so cxx generates no shared struct definitions.

#[cxx::bridge(namespace = "torrentfish")]
pub mod ffi {
    unsafe extern "C++" {
        include!("torrent_fish_shim.h");

        type Session;

        #[allow(clippy::too_many_arguments)]
        fn new_session(
            user_agent: &str,
            listen_interfaces: &str,
            download_rate_limit: i64,
            upload_rate_limit: i64,
            connections_limit: i64,
            outgoing_interfaces: &str,
            has_proxy: bool,
            proxy_hostname: &str,
            proxy_port: u16,
        ) -> UniquePtr<Session>;

        fn add_torrent(self: Pin<&mut Session>, path: &str, save_path: &str) -> Result<String>;
        fn pop_alerts(self: &Session) -> Vec<String>;
        fn listen_port(self: &Session) -> u16;

        fn upload_mode(self: &Session) -> bool;
        fn auto_managed(self: &Session) -> bool;
        fn paused(self: &Session) -> bool;
        fn has_metadata(self: &Session) -> bool;
        fn total_wanted(self: &Session) -> i64;
        fn total_download(self: &Session) -> i64;
        fn total_payload_download(self: &Session) -> i64;
        fn total_upload(self: &Session) -> i64;
        fn num_pieces(self: &Session) -> u32;

        fn libtorrent_version() -> String;

        fn probe_metadata(torrent_path: &str, peer_port: u16, timeout_ms: i64) -> Result<String>;
    }
}
