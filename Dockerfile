FROM ubuntu:22.04
LABEL maintainer="Chris Partridge <chris@partridge.tech>"

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

USER root
WORKDIR /opt

ENV LANG=en_US.UTF-8
ENV LC_ALL=C.UTF-8
ENV LANG=C.UTF-8

# install Python and libtorrent
RUN ln -snf "/usr/share/zoneinfo/UTC" /etc/localtime && echo "UTC" > /etc/timezone && \
    apt-get update -y && \
    apt-get install -y \
        python3 \
        python3-libtorrent && \
    # save some space 
    rm -rf /var/lib/apt/lists/*

# create less-privileged user
RUN useradd --create-home fish && echo "fish:fish" | chpasswd

# copy torrent-fish.py into Docker
COPY torrent-fish.py /home/fish/torrent-fish.py

# set environment
USER fish
ENTRYPOINT ["python3", "-u", "/home/fish/torrent-fish.py"]