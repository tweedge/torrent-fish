# Build stage: install the libtorrent wheel into site-packages.
# A full toolchain image is only needed here; none of it ships in the final image.
FROM python:3.11-slim AS builder

RUN pip install --no-cache-dir libtorrent==2.1.1

# Keep build-only tooling out of the runtime image.
RUN rm -rf /usr/local/lib/python3.11/site-packages/pip* \
    /usr/local/lib/python3.11/site-packages/setuptools* \
    /usr/local/lib/python3.11/site-packages/wheel*

# Runtime stage: distroless Python 3.11.
# This image is built on distroless's "cc" base, which provides the libstdc++
# and libgcc shared libraries that the libtorrent extension links against.
# The :nonroot tag runs as uid 65532 and defaults to /home/nonroot.
FROM gcr.io/distroless/python3-debian12:nonroot

# Bring in the installed packages from the build stage.
COPY --from=builder /usr/local/lib/python3.11/site-packages \
    /usr/local/lib/python3.11/site-packages

# Without pip present, site-packages is not on sys.path by default.
ENV PYTHONPATH=/usr/local/lib/python3.11/site-packages
# Unbuffered output so logs stream, and no .pyc writes against the read-only paths.
ENV PYTHONUNBUFFERED=1
ENV PYTHONDONTWRITEBYTECODE=1

COPY --chown=nonroot:nonroot torrent-fish.py /home/nonroot/torrent-fish.py

USER nonroot:nonroot
WORKDIR /home/nonroot

ENTRYPOINT ["python3", "-u", "/home/nonroot/torrent-fish.py"]
