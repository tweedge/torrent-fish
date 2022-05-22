# torrent-fish

[![Pulls](https://img.shields.io/docker/pulls/tweedge/torrent-fish)](https://hub.docker.com/repository/docker/tweedge/torrent-fish)
[![License](https://img.shields.io/github/license/tweedge/unishox2-py3)](https://github.com/tweedge/unishox2-py3)
[![Code Style](https://img.shields.io/badge/code%20style-black-black)](https://github.com/psf/black)

torrent-fish is a *metadata-only* BitTorrent peer built with libtorrent. It's designed to help keep magnet links which rely on [HTTP Seeding](https://wiki.vuze.com/w/HTTP_Seeding) healthy, without the overhead of running other peers or seeds.

### Torrenting for Archivists

BitTorrent is fantastically thrifty for large groups of people looking to share specific files online, with each contributing their own bandwidth and resources to provide the files to each other person who is requesting those files. The original uploader does not even bear exclusive (or even the majority) cost for distributing files, and popular torrents can live for years after the original uploader stopped seeding their own file.

That doesn't mean that BitTorrent is the economic choice for all situations, though. BitTorrent expects clients in swarms simultaneously to exchange files - as such, it's rarely a choice for archival storage. A seeder looking at the long term needs to be online all the time, with reasonable frequency (if permitted for their torrent's availability needs), or have some mechanism of determining when it needs to be woken up. These options are pricy, inconvenient, or both.

One way to address this is by using [GetRight-style HTTP Seeding](https://www.bittorrent.org/beps/bep_0019.html), which instructs torrent clients to treat certain HTTP or FTP sources as seeds for a given torrent. That way, no dedicated system is needed to serve the content of a torrent - especially if the HTTP Seed is using now-commodity object storage or a CDN - which can lower cost *in certain situations* compared to even the cheapest virtual machine serving the torrent 24/7.

However, this has a catch. Torrents utilizing HTTP Seeds is supported end-to-end. Magnets utilizing webseeds are **not** supported end-to-end, the [magnet specification](https://www.bittorrent.org/beps/bep_0009.html) *relies* on a swarm being alive to provision new peers with the torrent; neither [webseeds](https://www.bittorrent.org/beps/bep_0017.html) nor GetRight (above) will fetch torrent metadata, even if a healthy HTTP Seed is available, because there is no defined location that the client should query to fetch a copy of the torrent (therefore, the content is unknown to the client - it cannot know what to query the HTTP Seed for until it knows what filenames and byteranges to fetch).

*An aside: this is [how the Internet Archive distributes data via torrents](https://help.archive.org/help/archive-bittorrents/), they specifically use torrents + HTTP Seeds for the above reasons. A torrent file is provided for for downloads, which allows HTTP Seeds to be utilized, plus anyone else in the swarm can donate bandwidth to help distribute content. The Internet Archive does not provide magnets, as those cannot be converted to torrents without running a torrent client, which is expensive.*

But, torrents are not the best option for every situation - magnet links are much smaller and can be posted wherever text input is allowed, so they show up on many more websites than torrents do.

torrent-fish provides a solution to this (very niche) issue: if someone has a magnet link with an HTTP Seed, they still need a peer to get the torrent file from. If the swarm is reasonably expected to be empty at least sometimes, ex. for long-term distribution of low-interest or large torrents which makes staying in the swarm cost-prohibitive, torrent-fish is a lightweight script which performs the only essential task of a peer in that swarm: provide the torrent metadata to any new peers so they may use the HTTP Seed(s) to retrieve the content itself.

### Illustrate the Problem!

#### 1. Here's a normal swarm with HTTP Seeds works well!

![Image 1](https://github.com/tweedge/torrent-fish/blob/main/diagrams/1_normal_swarm_with_http_seeds.png?raw=true)

#### 2. But if a client has a magnet and nobody is in the swarm to provision them with a copy of the torrent, they can't fetch any content and stall. This is bad!

![Image 2](https://github.com/tweedge/torrent-fish/blob/main/diagrams/2_client_has_magnet_with_only_http_seeds.png?raw=true)

#### 3. If the client already has the torrent, they don't need a swarm at all though! Fetching all other data from the HTTP Seed is possible.

![Image 3](https://github.com/tweedge/torrent-fish/blob/main/diagrams/3_client_has_torrent_with_only_http_seeds.png?raw=true)

#### 4. This is what torrent-fish ensures: if a client has a magnet, there will always be at least one peer in the swarm to give them a copy of the torrent. It should be cheap, easy, and reliable.

![Image 4](https://github.com/tweedge/torrent-fish/blob/main/diagrams/4_client_has_magnet_with_torrentfish_and_http_seeds.png?raw=true)

### How does torrent-fish work?

[libtorrent](https://www.libtorrent.org/) is a popular library that implements BitTorrent - you might use a BitTorrent client that also uses libtorrent, such as [Deluge](https://deluge-torrent.org/) or [qBittorrent](https://www.qbittorrent.org/). torrent-fish wraps libtorrent with pretty simple settings applied to every torrent:

* Each piece's priority is set to 0 (do not download)
* `upload_mode` flag is enabled, so torrents are active but not downloading data
* `auto_managed` flag is disabled, so torrents should not enter a downloading state automatically

It should be possible to accomplish the desired results by using *either* setting the piece priorities *or* mucking about with the flags for each torrent, but I occasionally ran into issues where libtorrent would begin to request pieces (BitTorrent is supposed to download missing/corrupt pieces, after all!) and doing both doesn't seem to cause any problems.

The script was originally a fork of François M.'s [retrieve.py](https://gist.github.com/francoism90/4db9efa5af546d831ca47208e58f3364) due to his clean usage of [libtorrent's bindings for Python](https://www.libtorrent.org/python_binding.html), though we accomplish very different goals with libtorrent. Francois' script gets a torrent from a magnet and exits; torrent-fish stays alive for as long as possible to convert any peer's magnet into a torrent.

### Efficiency of torrent-fish

The Docker container for torrent-fish is very small, only about 40MB compressed. When run in a [Firecracker microVM](https://firecracker-microvm.github.io/) on [Fly](https://fly.io/), the entire torrent-fish instance consumes about 80MB of RAM and several KB/s of bandwidth. This fits easily into Fly's smallest current offering - 1 core, 256MB RAM - *and* customers currently get three of those instances free-forever.
