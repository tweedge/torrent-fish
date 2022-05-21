# torrent-fish

[![Pulls](https://img.shields.io/docker/pulls/tweedge/torrent-fish)](https://hub.docker.com/repository/docker/tweedge/torrent-fish)
[![License](https://img.shields.io/github/license/tweedge/unishox2-py3)](https://github.com/tweedge/unishox2-py3)
[![Code Style](https://img.shields.io/badge/code%20style-black-black)](https://github.com/psf/black)

torrent-fish is a *metadata-only* BitTorrent peer built with libtorrent. It's designed to help keep torrents and magnets (but *especially magnets*) which rely on [HTTP Seeding](https://wiki.vuze.com/w/HTTP_Seeding) healthy, without the overhead of running other peers or seeds.

### Torrenting for Archivists

BitTorrent expects clients in swarms simultaneously to exchange files, so a seeder looking at the long term needs to be online all the time, with enough regularity, or have some mechanism of determining when it needs to be woken up.

Pricy.

Storage as commodity, with no servers, and especially with a CDN you can get preposterous scale with little effort and expense. Can we use it?

Yes, with some gaps. Torrents -> webseeds is supported end-to-end. Magnets -> webseeds are not, the magnet specification *relies* on a swarm being alive. This is why IA distributes torrents instead of magnets, they can skip that deficiency.

For rogue archivists, you can slap a webseed/getright tag on any existing magnet (or create a new magnet, etc.) and clients that use your amended magnet will accept the HTTP Seed as a new source - but that means you need to keep a peer online. Enter torrent-fish - staying connected to swarms, to perform the only essential duty of a peer, with the least possible cost.

### Under the Hood

[libtorrent](https://www.libtorrent.org/) is a popular library that implements BitTorrent - you might use a BitTorrent client that also uses libtorrent, such as [Deluge](https://deluge-torrent.org/) or [qBittorrent](https://www.qbittorrent.org/). torrent-fish wraps libtorrent with pretty simple settings applied to every torrent:

* Each piece's priority is set to 0 (do not download)
* `upload_mode` flag is enabled, so torrents are active but not download data
* `auto_managed` flag is disabled, so torrents may not enter a downloading state automatically

It should be possible to accomplish the desired results by using *either* setting the piece priorities *or* mucking about with the flags for each torrent, but I occasionally ran into issues where libtorrent would begin to request pieces (BitTorrent is supposed to download missing/corrupt pieces, after all!) and doing both doesn't seem to cause any problems.

The script was originally a fork of François M.'s [retrieve.py](https://gist.github.com/francoism90/4db9efa5af546d831ca47208e58f3364) due to his clean usage of [libtorrent's bindings for Python](https://www.libtorrent.org/python_binding.html), though we accomplish very different goals with libtorrent. Francois' script gets a torrent from a magnet and exits; torrent-fish stays alive for as long as possible to convert any peer's magnet into a torrent.

### Resource Efficiency

The Docker container for torrent-fish is very small, only about 40MB compressed. When run in a [Firecracker microVM](https://firecracker-microvm.github.io/) on [Fly](https://fly.io/), the entire torrent-fish instance consumes about 80MB of RAM and several KB/s of bandwidth. This fits easily into Fly's smallest current offering - 1 core, 256MB RAM - *and* customers currently get three of those instances free-forever.
