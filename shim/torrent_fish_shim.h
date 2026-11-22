#pragma once

#include "rust/cxx.h"
#include <cstdint>
#include <memory>

// Thin CXX bridge over the small slice of libtorrent that torrent-fish uses.
// The interface deliberately uses only primitives and rust::String/Vec so cxx
// generates no shared type definitions and there is no include cycle. All
// torrent-fish policy (CLI parsing, file discovery, conversions, skip-on-error)
// stays in Rust.
namespace torrentfish {

class Session {
 public:
  Session(::rust::Str user_agent, ::rust::Str listen_interfaces,
          ::std::int64_t download_rate_limit, ::std::int64_t upload_rate_limit,
          ::std::int64_t connections_limit, ::rust::Str outgoing_interfaces,
          bool has_proxy, ::rust::Str proxy_hostname, ::std::uint16_t proxy_port);
  ~Session();

  // Adds a .torrent file and configures it as metadata-only (upload_mode on,
  // auto_managed off, every piece priority 0, then resumed). Returns the
  // torrent's display name on success, throws on invalid/corrupt torrents.
  ::rust::String add_torrent(::rust::Str path, ::rust::Str save_path);

  // Drains all pending alert messages.
  ::rust::Vec<::rust::String> pop_alerts() const;

  // Port the session actually listens on (0 if none).
  ::std::uint16_t listen_port() const;

  // Inspection of the first added torrent (tests/metrics).
  bool upload_mode() const;
  bool auto_managed() const;
  bool paused() const;
  bool has_metadata() const;
  ::std::int64_t total_wanted() const;
  ::std::int64_t total_download() const;
  ::std::int64_t total_payload_download() const;
  ::std::int64_t total_upload() const;
  ::std::uint32_t num_pieces() const;

 private:
  struct Impl;
  ::std::unique_ptr<Impl> impl_;
};

::std::unique_ptr<Session> new_session(
    ::rust::Str user_agent, ::rust::Str listen_interfaces,
    ::std::int64_t download_rate_limit, ::std::int64_t upload_rate_limit,
    ::std::int64_t connections_limit, ::rust::Str outgoing_interfaces,
    bool has_proxy, ::rust::Str proxy_hostname, ::std::uint16_t proxy_port);

// libtorrent version string, for logging parity with the Python user agent.
::rust::String libtorrent_version();

// Test/probe helper: act as a magnet-only BitTorrent client that fetches the
// metadata for the torrent at `torrent_path` from a peer at 127.0.0.1:peer_port
// (i.e. a torrent-fish instance). Network discovery is disabled so the fish peer
// is the only source. Returns the torrent's name on success and throws if the
// metadata is not received within `timeout_ms`. Used to prove, without Python,
// that torrent-fish serves metadata.
::rust::String probe_metadata(::rust::Str torrent_path, ::std::uint16_t peer_port,
                              ::std::int64_t timeout_ms);

}  // namespace torrentfish
