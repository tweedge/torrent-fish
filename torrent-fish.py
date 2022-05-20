#!/usr/bin/env python3

# refer:
# https://gist.github.com/francoism90/4db9efa5af546d831ca47208e58f3364
# https://github.com/arvidn/libtorrent/issues/2239

import libtorrent as lt
import time
import json
import os


def get_absolute_paths_to_files_in_directory(directory):
    abspath_to_files = []
    for root, dirs, files in os.walk(os.path.abspath(directory)):
        for file in files:
            abspath_to_files.append(os.path.join(root, file))
    return abspath_to_files


def add_torrent(ses, filename, options):
    atp = lt.add_torrent_params()

    info = lt.torrent_info(filename)
    atp.ti = info

    atp.save_path = options.save_path
    atp.storage_mode = lt.storage_mode_t.storage_mode_sparse
    atp.flags |= lt.torrent_flags.upload_mode

    handle = ses.add_torrent(atp)

    for i in range(info.num_pieces()):
        handle.piece_priority(i, 0)


def get_file_info(file):
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

    entry = {}

    for attribute in attributes:
        entry[attribute] = getattr(file, attribute, None)

    return entry


def main():
    from optparse import OptionParser

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
        "-s",
        "--save-path",
        type="string",
        help="the path where the downloaded file/folder should be placed.",
    )

    parser.add_option(
        "-l",
        "--load-path",
        type="string",
        help="the path where the downloaded file/folder should be placed.",
    )

    parser.add_option(
        "-r",
        "--proxy-host",
        type="string",
        help="sets HTTP proxy host and port (separated by ':')",
    )

    parser.set_defaults(
        port=6881,
        listen_interface="0.0.0.0",
        outgoing_interface="",
        max_download_rate=0,
        max_upload_rate=0,
        connections_limit=800,  # limit increased
        save_path="/tmp",
        load_path="/opt",
        proxy_host="",
    )

    (options, args) = parser.parse_args()

    if options.port < 0 or options.port > 65525:
        options.port = 6881

    options.max_upload_rate *= 1000
    options.max_download_rate *= 1000

    if options.max_upload_rate <= 0:
        options.max_upload_rate = -1
    if options.max_download_rate <= 0:
        options.max_download_rate = -1

    settings = {
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
    }

    if options.proxy_host != "":
        settings["proxy_hostname"] = options.proxy_host.split(":")[0]
        settings["proxy_type"] = lt.proxy_type_t.http
        settings["proxy_port"] = options.proxy_host.split(":")[1]

    ses = lt.session(settings)

    torrent_files = get_absolute_paths_to_files_in_directory(options.load_path)
    if len(torrent_files) < 1:
        print("[torrent-fish] No files found. Quitting ...")
        exit(1)

    for torrent_file in torrent_files:
        print("[torrent-fish] Adding %s" % (torrent_file))
        add_torrent(ses, torrent_file, options)

    # fetch and display debug logs
    while True:
        alerts = ses.pop_alerts()
        for a in alerts:
            print(f"[libtorrent] {a}")

        time.sleep(0.1)


main()
