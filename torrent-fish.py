#!/usr/bin/env python3

# refer:
# https://gist.github.com/francoism90/4db9efa5af546d831ca47208e58f3364
# https://github.com/arvidn/libtorrent/issues/2239

from __future__ import annotations

import os
import time
from optparse import OptionParser, Values
from typing import Any, Dict, List, Optional, Tuple

import libtorrent as lt

DEFAULT_PORT = 6881
DEFAULT_PROXY_PORT = 8080


def get_absolute_paths_to_files_in_directory(directory: str) -> List[str]:
    abspath_to_files: List[str] = []
    for root, dirs, files in os.walk(os.path.abspath(directory)):
        for file in files:
            abspath_to_files.append(os.path.join(root, file))
    return abspath_to_files


def sanitize_port(port: int) -> int:
    """Return a usable TCP port, falling back to the default when out of range."""
    if port < 1 or port > 65535:
        return DEFAULT_PORT
    return port


def convert_rate_limit(rate_kbps: float) -> int:
    """Convert a rate in kB/s to libtorrent's bytes/s limit (-1 means infinite)."""
    rate = int(rate_kbps * 1000)
    return rate if rate > 0 else -1


def parse_proxy_host(proxy_host: str) -> Optional[Tuple[str, int]]:
    """Parse a 'host[:port]' proxy string into a (host, port) tuple."""
    if proxy_host == "":
        return None
    host, _, port = proxy_host.partition(":")
    try:
        port_num = int(port)
    except ValueError:
        port_num = DEFAULT_PROXY_PORT
    if port_num < 1 or port_num > 65535:
        port_num = DEFAULT_PROXY_PORT
    return (host, port_num)


def add_torrent(ses: lt.session, filename: str, save_path: str) -> lt.torrent_handle:
    atp = lt.add_torrent_params()

    info = lt.torrent_info(filename)
    atp.ti = info

    atp.save_path = save_path
    atp.storage_mode = lt.storage_mode_t.storage_mode_sparse
    atp.flags |= lt.torrent_flags.upload_mode
    atp.flags &= ~lt.torrent_flags.auto_managed

    handle = ses.add_torrent(atp)

    for i in range(info.num_pieces()):
        handle.piece_priority(i, 0)

    # clearing auto_managed leaves the torrent paused, so resume it explicitly
    handle.resume()

    return handle


def load_torrents(ses: lt.session, torrent_files: List[str], save_path: str) -> int:
    """Add every valid torrent file, skipping (and reporting) invalid ones."""
    added = 0
    for torrent_file in torrent_files:
        print("[torrent-fish] Adding %s" % (torrent_file))
        try:
            add_torrent(ses, torrent_file, save_path)
            added += 1
        except (RuntimeError, ValueError) as e:
            print("[torrent-fish] Skipping %s: %s" % (torrent_file, e))
    return added


def get_file_info(file: Any) -> Dict[str, Any]:
    attributes = [
        "path",
        "symlink_path",
        "offset",
        "size",
        "mtime",
        "filehash",
        "pad_file",
        "hidden_attribute",
        "executable_attribute",
        "symlink_attribute",
    ]

    entry: Dict[str, Any] = {}

    for attribute in attributes:
        entry[attribute] = getattr(file, attribute, None)

    return entry


def build_settings(options: Values) -> Dict[str, Any]:
    settings: Dict[str, Any] = {
        "user_agent": "libtorrent/%s (https://torrent.fish/)" % (lt.__version__),
        "listen_interfaces": "%s:%d" % (options.listen_interface, options.port),
        "download_rate_limit": int(options.max_download_rate),
        "upload_rate_limit": int(options.max_upload_rate),
        "connections_limit": int(options.connections_limit),
        "dht_bootstrap_nodes": "router.bittorrent.com:6881,dht.transmissionbt.com:6881,router.utorrent.com:6881,",
        "outgoing_interfaces": options.outgoing_interface,
        "announce_to_all_tiers": True,
        "announce_to_all_trackers": True,
        "auto_manage_interval": -1,
        "auto_scrape_interval": 1800,  # increased to default
        "auto_scrape_min_interval": 300,  # increased to default
        "max_failcount": 1,
        "aio_threads": 8,
        "checking_mem_usage": 2048,
        "enable_upnp": False,
    }

    proxy = parse_proxy_host(options.proxy_host)
    if proxy is not None:
        settings["proxy_hostname"] = proxy[0]
        settings["proxy_type"] = lt.proxy_type_t.http
        settings["proxy_port"] = proxy[1]

    return settings


def main() -> None:
    parser = OptionParser()

    parser.add_option("-p", "--port", type="int", help="set listening port")

    parser.add_option(
        "-i",
        "--listen-interface",
        type="string",
        help="set interface for incoming connections",
    )

    parser.add_option(
        "-o",
        "--outgoing-interface",
        type="string",
        help="set interface for outgoing connections",
    )

    parser.add_option(
        "-d",
        "--max-download-rate",
        type="float",
        help="the maximum download rate given in kB/s. 0 means infinite.",
    )

    parser.add_option(
        "-u",
        "--max-upload-rate",
        type="float",
        help="the maximum upload rate given in kB/s. 0 means infinite.",
    )

    parser.add_option(
        "-c",
        "--connections_limit",
        type="int",
        help="the global limit on the number of connections opened.",
    )

    parser.add_option(
        "-l",
        "--load-path",
        type="string",
        help="the path to load torrent files from.",
    )

    parser.add_option(
        "-s",
        "--save-path",
        type="string",
        help="must be a valid folder BUT won't be used to download anything.",
    )

    parser.add_option(
        "-r",
        "--proxy-host",
        type="string",
        help="sets HTTP proxy host and port (separated by ':')",
    )

    parser.set_defaults(
        port=DEFAULT_PORT,
        listen_interface="0.0.0.0",
        outgoing_interface="",
        max_download_rate=0,
        max_upload_rate=0,
        connections_limit=800,  # limit increased
        save_path="/opt",  # this should(tm) never be used for storage
        load_path="/opt",
        proxy_host="",
    )

    (options, args) = parser.parse_args()

    options.port = sanitize_port(options.port)
    options.max_upload_rate = convert_rate_limit(options.max_upload_rate)
    options.max_download_rate = convert_rate_limit(options.max_download_rate)

    settings = build_settings(options)
    # libtorrent's Python bindings accept a plain dict here at runtime, but its
    # type stubs only model the (unexposed) settings_pack overload.
    ses = lt.session(settings)  # type: ignore[call-overload]

    torrent_files = get_absolute_paths_to_files_in_directory(options.load_path)
    if len(torrent_files) < 1:
        print("[torrent-fish] No files found. Quitting ...")
        exit(1)

    added = load_torrents(ses, torrent_files, options.save_path)
    if added < 1:
        print("[torrent-fish] No valid torrents found. Quitting ...")
        exit(1)

    # fetch and display debug logs
    while True:
        alerts = ses.pop_alerts()
        for a in alerts:
            print(f"[libtorrent] {a}")

        time.sleep(0.1)


if __name__ == "__main__":
    main()
