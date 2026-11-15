import os
import time

import pytest

lt = pytest.importorskip("libtorrent")


def _flags(handle):
    return handle.flags()


def test_add_torrent_sets_upload_mode(torrent_fish, session, torrent_file, tmp_path):
    torrent_path, _ = torrent_file
    handle = torrent_fish.add_torrent(session, torrent_path, str(tmp_path / "save"))

    flags = _flags(handle)
    assert flags & lt.torrent_flags.upload_mode


def test_add_torrent_disables_auto_managed(
    torrent_fish, session, torrent_file, tmp_path
):
    torrent_path, _ = torrent_file
    handle = torrent_fish.add_torrent(session, torrent_path, str(tmp_path / "save"))

    assert not (_flags(handle) & lt.torrent_flags.auto_managed)


def test_add_torrent_is_not_paused(torrent_fish, session, torrent_file, tmp_path):
    torrent_path, _ = torrent_file
    handle = torrent_fish.add_torrent(session, torrent_path, str(tmp_path / "save"))

    time.sleep(0.5)
    # If auto_managed is cleared without an explicit resume, libtorrent leaves
    # the torrent paused and it will not accept metadata requests.
    assert not handle.status().paused


def test_add_torrent_sets_all_piece_priorities_zero(
    torrent_fish, session, torrent_file, tmp_path
):
    torrent_path, _ = torrent_file
    handle = torrent_fish.add_torrent(session, torrent_path, str(tmp_path / "save"))

    priorities = handle.piece_priorities()
    assert priorities
    assert all(p == 0 for p in priorities)


def test_add_torrent_wants_no_data(torrent_fish, session, torrent_file, tmp_path):
    torrent_path, _ = torrent_file
    handle = torrent_fish.add_torrent(session, torrent_path, str(tmp_path / "save"))

    time.sleep(0.5)
    status = handle.status()
    assert status.has_metadata
    assert status.total_wanted == 0


def test_add_torrent_returns_handle_with_metadata(
    torrent_fish, session, torrent_file, tmp_path
):
    torrent_path, info_hash = torrent_file
    handle = torrent_fish.add_torrent(session, torrent_path, str(tmp_path / "save"))

    time.sleep(0.5)
    assert handle.status().has_metadata
    assert handle.torrent_file() is not None


def test_add_torrent_does_not_write_content(
    torrent_fish, session, torrent_file, tmp_path
):
    torrent_path, _ = torrent_file
    save_path = tmp_path / "save"
    save_path.mkdir()
    torrent_fish.add_torrent(session, torrent_path, str(save_path))

    time.sleep(0.5)
    written = [
        name
        for name in os.listdir(save_path)
        if name != ".pad" and not name.startswith(".")
    ]
    assert written == []


def test_load_torrents_skips_invalid_files(
    torrent_fish, session, torrent_file, tmp_path, capsys
):
    torrent_path, _ = torrent_file
    junk = tmp_path / "junk.txt"
    junk.write_text("not a torrent")

    added = torrent_fish.load_torrents(
        session, [torrent_path, str(junk)], str(tmp_path / "save")
    )

    assert added == 1
    captured = capsys.readouterr()
    assert "Skipping" in captured.out


def test_load_torrents_counts_only_valid(torrent_fish, session, torrent_file, tmp_path):
    torrent_path, _ = torrent_file
    added = torrent_fish.load_torrents(session, [torrent_path], str(tmp_path / "save"))
    assert added == 1


def test_load_torrents_all_invalid_returns_zero(torrent_fish, session, tmp_path):
    junk = tmp_path / "bad.torrent"
    junk.write_bytes(b"\x00\x01not bencoded")
    added = torrent_fish.load_torrents(session, [str(junk)], str(tmp_path / "save"))
    assert added == 0


def test_torrent_serves_metadata_to_magnet_peer(
    torrent_fish, session, torrent_file, tmp_path
):
    """End-to-end: a magnet-only peer obtains metadata from the fish handle."""
    torrent_path, _ = torrent_file
    handle = torrent_fish.add_torrent(session, torrent_path, str(tmp_path / "save"))
    time.sleep(0.5)

    port = session.listen_port()
    assert port > 0, "fish session is not listening"

    ti = lt.torrent_info(torrent_path)
    magnet = lt.make_magnet_uri(ti)

    peer_session = lt.session(
        {
            "listen_interfaces": "127.0.0.1:0",
            "enable_dht": False,
            "enable_lsd": False,
            "enable_upnp": False,
            "enable_natpmp": False,
        }
    )
    atp = lt.parse_magnet_uri(magnet)
    atp.save_path = str(tmp_path / "peer")
    atp.peers = [("127.0.0.1", port)]
    peer = peer_session.add_torrent(atp)

    deadline = time.time() + 20
    while time.time() < deadline:
        if peer.status().has_metadata:
            break
        time.sleep(0.2)

    assert peer.status().has_metadata, "peer never received metadata from torrent-fish"
