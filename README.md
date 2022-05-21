# torrent-fish

[![Pulls](https://img.shields.io/docker/pulls/tweedge/torrent-fish)](https://hub.docker.com/repository/docker/tweedge/torrent-fish)
[![License](https://img.shields.io/github/license/tweedge/unishox2-py3)](https://github.com/tweedge/unishox2-py3)
[![Code Style](https://img.shields.io/badge/code%20style-black-black)](https://github.com/psf/black)

torrent-fish is a cheap-to-run, metadata-only BitTorrent peer built with libtorrent.

When provisioned with torrent files, torrent-fish will join each swarm and happily send the torrent files to any peer that requests them (ex. peers with a magnet link that need the torrent).  **torrent-fish will *not* download or upload content from the torrents you give it and does *not* speed up torrent transfers.** Each piece has its priority set to zero, so torrent-fish will not download any pieces. If any piece is requested from torrent-fish by a peer, torrent-fish will have no data to reply with.

So, what's the point?

### Torrenting for Archivists

BitTorrent expects clients in swarms simultaneously to exchange files, so a seeder looking at the long term needs to be online all the time, with enough regularity, or have some mechanism of determining when it needs to be woken up.

Pricy.

Storage as commodity, with no servers, and especially with a CDN you can get preposterous scale with little effort and expense. Can we use it?

Yes, with some gaps. Torrents -> webseeds is supported end-to-end. Magnets -> webseeds are not, the magnet specification *relies* on a swarm being alive. This is why IA distributes torrents instead of magnets, they can skip that deficiency.

For rogue archivists, you can slap a webseed/getright tag on any existing magnet (or create a new magnet, etc.) and clients that use your amended magnet will accept the HTTP Seed as a new source - but that means you need to keep a peer online. Enter torrent-fish - staying connected to swarms, to perform the only essential duty of a peer, with the least possible cost.

### Resource Efficiency

The Docker container for torrent-fish is very small, only about 40MB compressed. When run in a [Firecracker microVM](https://firecracker-microvm.github.io/) on [Fly](https://fly.io/), the entire torrent-fish instance consumes about 80MB of RAM and several KB/s of bandwidth. This fits easily into Fly's smallest current offering - 1 core, 256MB RAM - *and* customers currently get three of those instances free-forever.