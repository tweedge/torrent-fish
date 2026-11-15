import importlib.util
import os
import sys

import pytest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCRIPT_PATH = os.path.join(REPO_ROOT, "torrent-fish.py")


def _load_torrent_fish():
    spec = importlib.util.spec_from_file_location("torrent_fish", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="session")
def torrent_fish():
    """Import torrent-fish.py (hyphenated, so loaded by path) as a module."""
    return _load_torrent_fish()


@pytest.fixture()
def torrent_file(tmp_path):
    """Create a valid .torrent on disk and return (torrent_path, info_hash)."""
    lt = pytest.importorskip("libtorrent")
    content_dir = tmp_path / "content"
    content_dir.mkdir()
    (content_dir / "hello.txt").write_text("torrent-fish test payload\n" * 20)

    fs = lt.file_storage()
    lt.add_files(fs, str(content_dir))
    create = lt.create_torrent(fs, piece_size=16384)
    lt.set_piece_hashes(create, str(tmp_path))

    torrent_path = tmp_path / "test.torrent"
    torrent_path.write_bytes(lt.bencode(create.generate()))

    info = lt.torrent_info(str(torrent_path))
    return str(torrent_path), info.info_hash()


@pytest.fixture()
def session():
    """A minimal, network-disabled libtorrent session for local tests."""
    lt = pytest.importorskip("libtorrent")
    return lt.session(
        {
            "listen_interfaces": "127.0.0.1:0",
            "enable_dht": False,
            "enable_lsd": False,
            "enable_upnp": False,
            "enable_natpmp": False,
        }
    )
