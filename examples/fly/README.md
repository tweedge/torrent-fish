# torrent-fish/examples/fly

This is not really a how-to-use-Fly tutorial, and I am not guaranteeing that the included Dockerfile or fly.toml are going to be kept up-to-date.

However, this was all I needed to:

* Pack any torrents in a local directory creatively titled "torrents,"
* Build them into a container automatically using Fly,
* Launch a copy of torrent-fish that automatically registers as a peer to every tracker, DHT, etc., and
* Serves the included torrent metadata for a measly $2-4/mo
  * If you need a public IPv4 address - and you would to support IPv4 clients - then you'd need to pony up an extra fee

...as of April 2023.

Assuming it's up-to-date-*enough*, you should be able to use these artifacts alongside Fly's documentation on [deploying an application via Dockerfile](https://fly.io/docs/getting-started/dockerfile/) to bootstrap your own online-forever copy of torrent-fish pretty quickly!
