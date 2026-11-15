import os

import pytest


def test_finds_files_non_recursively(torrent_fish, tmp_path):
    (tmp_path / "a.torrent").write_bytes(b"x")
    (tmp_path / "b.torrent").write_bytes(b"y")

    found = torrent_fish.get_absolute_paths_to_files_in_directory(str(tmp_path))
    assert sorted(os.path.basename(p) for p in found) == ["a.torrent", "b.torrent"]
    assert all(os.path.isabs(p) for p in found)


def test_finds_files_recursively(torrent_fish, tmp_path):
    nested = tmp_path / "a" / "b"
    nested.mkdir(parents=True)
    (nested / "deep.torrent").write_bytes(b"x")
    (tmp_path / "top.torrent").write_bytes(b"y")

    found = torrent_fish.get_absolute_paths_to_files_in_directory(str(tmp_path))
    assert sorted(os.path.basename(p) for p in found) == ["deep.torrent", "top.torrent"]


def test_missing_directory_returns_empty(torrent_fish, tmp_path):
    found = torrent_fish.get_absolute_paths_to_files_in_directory(
        str(tmp_path / "does-not-exist")
    )
    assert found == []


@pytest.mark.parametrize(
    "port,expected",
    [
        (-1, 6881),
        (0, 6881),
        (1, 1),
        (6881, 6881),
        (65535, 65535),
        (65536, 6881),
        (70000, 6881),
    ],
)
def test_sanitize_port(torrent_fish, port, expected):
    assert torrent_fish.sanitize_port(port) == expected


@pytest.mark.parametrize(
    "rate,expected",
    [
        (0, -1),
        (-5, -1),
        (1, 1000),
        (100, 100000),
        (2.5, 2500),
    ],
)
def test_convert_rate_limit(torrent_fish, rate, expected):
    assert torrent_fish.convert_rate_limit(rate) == expected


@pytest.mark.parametrize(
    "value,expected",
    [
        ("", None),
        ("1.2.3.4:8080", ("1.2.3.4", 8080)),
        ("proxy.example.com:3128", ("proxy.example.com", 3128)),
        ("badproxy", ("badproxy", 8080)),
        ("host:notaport", ("host", 8080)),
        ("host:", ("host", 8080)),
        (":8080", ("", 8080)),
        ("1.2.3.4:99999", ("1.2.3.4", 8080)),
        ("1.2.3.4:0", ("1.2.3.4", 8080)),
    ],
)
def test_parse_proxy_host(torrent_fish, value, expected):
    assert torrent_fish.parse_proxy_host(value) == expected


def test_build_settings_defaults(torrent_fish):
    import types

    options = types.SimpleNamespace(
        port=6881,
        listen_interface="0.0.0.0",
        outgoing_interface="",
        max_download_rate=-1,
        max_upload_rate=-1,
        connections_limit=800,
        proxy_host="",
    )
    settings = torrent_fish.build_settings(options)
    assert settings["listen_interfaces"] == "0.0.0.0:6881"
    assert settings["download_rate_limit"] == -1
    assert settings["upload_rate_limit"] == -1
    assert settings["connections_limit"] == 800
    assert "proxy_hostname" not in settings


def test_build_settings_with_proxy(torrent_fish):
    import types

    options = types.SimpleNamespace(
        port=1234,
        listen_interface="127.0.0.1",
        outgoing_interface="",
        max_download_rate=5000,
        max_upload_rate=7000,
        connections_limit=10,
        proxy_host="127.0.0.1:9999",
    )
    settings = torrent_fish.build_settings(options)
    assert settings["listen_interfaces"] == "127.0.0.1:1234"
    assert settings["proxy_hostname"] == "127.0.0.1"
    assert settings["proxy_port"] == 9999
    assert settings["download_rate_limit"] == 5000
    assert settings["upload_rate_limit"] == 7000
