#include "torrent_fish_shim.h"

#include <libtorrent/add_torrent_params.hpp>
#include <libtorrent/alert.hpp>
#include <libtorrent/session.hpp>
#include <libtorrent/settings_pack.hpp>
#include <libtorrent/torrent_flags.hpp>
#include <libtorrent/torrent_handle.hpp>
#include <libtorrent/torrent_info.hpp>
#include <libtorrent/version.hpp>

#include <chrono>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

namespace torrentfish {

namespace lt = libtorrent;

struct Session::Impl {
  lt::session session;
  bool has_torrent = false;
  explicit Impl(const lt::settings_pack& sp) : session(sp) {}
};

static lt::settings_pack build_settings_pack(
    rust::Str user_agent, rust::Str listen_interfaces,
    std::int64_t download_rate_limit, std::int64_t upload_rate_limit,
    std::int64_t connections_limit, rust::Str outgoing_interfaces,
    bool has_proxy, rust::Str proxy_hostname, std::uint16_t proxy_port) {
  lt::settings_pack sp;

  sp.set_str(lt::settings_pack::user_agent, std::string(user_agent));
  sp.set_str(lt::settings_pack::listen_interfaces, std::string(listen_interfaces));
  sp.set_int(lt::settings_pack::download_rate_limit,
             static_cast<int>(download_rate_limit));
  sp.set_int(lt::settings_pack::upload_rate_limit,
             static_cast<int>(upload_rate_limit));
  sp.set_int(lt::settings_pack::connections_limit,
             static_cast<int>(connections_limit));
  sp.set_str(lt::settings_pack::dht_bootstrap_nodes,
             "router.bittorrent.com:6881,dht.transmissionbt.com:6881,"
             "router.utorrent.com:6881,");
  sp.set_str(lt::settings_pack::outgoing_interfaces, std::string(outgoing_interfaces));
  sp.set_bool(lt::settings_pack::announce_to_all_tiers, true);
  sp.set_bool(lt::settings_pack::announce_to_all_trackers, true);
  sp.set_int(lt::settings_pack::auto_manage_interval, -1);
  sp.set_int(lt::settings_pack::auto_scrape_interval, 1800);
  sp.set_int(lt::settings_pack::auto_scrape_min_interval, 300);
  sp.set_int(lt::settings_pack::max_failcount, 1);
  sp.set_int(lt::settings_pack::aio_threads, 8);
  sp.set_int(lt::settings_pack::checking_mem_usage, 2048);
  sp.set_bool(lt::settings_pack::enable_upnp, false);

  if (has_proxy) {
    sp.set_str(lt::settings_pack::proxy_hostname, std::string(proxy_hostname));
    sp.set_int(lt::settings_pack::proxy_type, lt::settings_pack::http);
    sp.set_int(lt::settings_pack::proxy_port, static_cast<int>(proxy_port));
  }
  return sp;
}

Session::Session(rust::Str user_agent, rust::Str listen_interfaces,
                 std::int64_t download_rate_limit, std::int64_t upload_rate_limit,
                 std::int64_t connections_limit, rust::Str outgoing_interfaces,
                 bool has_proxy, rust::Str proxy_hostname, std::uint16_t proxy_port)
    : impl_(std::make_unique<Impl>(build_settings_pack(
          user_agent, listen_interfaces, download_rate_limit, upload_rate_limit,
          connections_limit, outgoing_interfaces, has_proxy, proxy_hostname,
          proxy_port))) {}

Session::~Session() = default;

rust::String Session::add_torrent(rust::Str path, rust::Str save_path) {
  auto ti = std::make_shared<lt::torrent_info>(std::string(path));

  lt::add_torrent_params atp;
  atp.ti = ti;
  atp.save_path = std::string(save_path);
  atp.storage_mode = lt::storage_mode_sparse;
  atp.flags |= lt::torrent_flags::upload_mode;
  atp.flags &= ~lt::torrent_flags::auto_managed;

  lt::torrent_handle handle = impl_->session.add_torrent(atp);

  const int num_pieces = ti->num_pieces();
  for (int i = 0; i < num_pieces; ++i) {
    handle.piece_priority(lt::piece_index_t(i), lt::download_priority_t(0));
  }

  // Clearing auto_managed leaves the torrent paused; resume it explicitly.
  handle.resume();

  impl_->has_torrent = true;
  return rust::String(ti->name());
}

rust::Vec<rust::String> Session::pop_alerts() const {
  rust::Vec<rust::String> out;
  std::vector<lt::alert*> alerts;
  impl_->session.pop_alerts(&alerts);
  for (lt::alert* a : alerts) {
    out.push_back(rust::String(a->message()));
  }
  return out;
}

std::uint16_t Session::listen_port() const { return impl_->session.listen_port(); }

static const lt::torrent_handle* first_handle(const lt::session& s) {
  // The vector is short-lived; caller keeps the session alive around use.
  static thread_local std::vector<lt::torrent_handle> handles;
  handles = s.get_torrents();
  if (handles.empty()) {
    return nullptr;
  }
  return &handles.front();
}

bool Session::upload_mode() const {
  auto* h = first_handle(impl_->session);
  return h && bool(h->flags() & lt::torrent_flags::upload_mode);
}
bool Session::auto_managed() const {
  auto* h = first_handle(impl_->session);
  return h && bool(h->flags() & lt::torrent_flags::auto_managed);
}
bool Session::paused() const {
  auto* h = first_handle(impl_->session);
  return h && bool(h->flags() & lt::torrent_flags::paused);
}
bool Session::has_metadata() const {
  auto* h = first_handle(impl_->session);
  return h && h->status().has_metadata;
}
std::int64_t Session::total_wanted() const {
  auto* h = first_handle(impl_->session);
  return h ? h->status().total_wanted : 0;
}
std::int64_t Session::total_download() const {
  auto* h = first_handle(impl_->session);
  return h ? h->status().total_download : 0;
}
std::int64_t Session::total_payload_download() const {
  auto* h = first_handle(impl_->session);
  return h ? h->status().total_payload_download : 0;
}
std::int64_t Session::total_upload() const {
  auto* h = first_handle(impl_->session);
  return h ? h->status().total_upload : 0;
}
std::uint32_t Session::num_pieces() const {
  auto* h = first_handle(impl_->session);
  if (!h || !h->torrent_file()) {
    return 0;
  }
  return static_cast<std::uint32_t>(h->torrent_file()->num_pieces());
}

std::unique_ptr<Session> new_session(
    rust::Str user_agent, rust::Str listen_interfaces,
    std::int64_t download_rate_limit, std::int64_t upload_rate_limit,
    std::int64_t connections_limit, rust::Str outgoing_interfaces, bool has_proxy,
    rust::Str proxy_hostname, std::uint16_t proxy_port) {
  return std::make_unique<Session>(user_agent, listen_interfaces,
                                   download_rate_limit, upload_rate_limit,
                                   connections_limit, outgoing_interfaces,
                                   has_proxy, proxy_hostname, proxy_port);
}

rust::String libtorrent_version() { return rust::String(lt::version()); }

rust::String probe_metadata(rust::Str torrent_path, std::uint16_t peer_port,
                            std::int64_t timeout_ms) {
  auto ti = std::make_shared<lt::torrent_info>(std::string(torrent_path));

  lt::settings_pack sp;
  sp.set_str(lt::settings_pack::listen_interfaces, "127.0.0.1:0");
  sp.set_bool(lt::settings_pack::enable_dht, false);
  sp.set_bool(lt::settings_pack::enable_lsd, false);
  sp.set_bool(lt::settings_pack::enable_upnp, false);
  sp.set_bool(lt::settings_pack::enable_natpmp, false);
  lt::session session(sp);

  lt::add_torrent_params atp;
  atp.info_hashes = ti->info_hashes();
  atp.save_path = "/tmp/tf-probe-dl";
  atp.peers.emplace_back(lt::make_address("127.0.0.1"), peer_port);
  atp.flags |= lt::torrent_flags::upload_mode;  // we only want metadata
  lt::torrent_handle h = session.add_torrent(atp);

  const auto deadline =
      std::chrono::steady_clock::now() + std::chrono::milliseconds(timeout_ms);
  while (std::chrono::steady_clock::now() < deadline) {
    if (h.status().has_metadata) {
      auto tf = h.torrent_file();
      return rust::String(tf ? tf->name() : std::string());
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
  }
  throw std::runtime_error("metadata not received from peer within timeout");
}

}  // namespace torrentfish
