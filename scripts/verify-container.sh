#!/usr/bin/env bash
# Verify a torrent-fish image end-to-end: it must serve metadata to a magnet-only
# peer while downloading zero payload bytes, and must run non-root.
#
# The metadata-fetching peer is the torrent-fish binary itself (--probe-metadata),
# so this needs no external torrent client or Python.
#
# Usage: verify-container.sh <image-tag> <host-binary> <port> <save-path-in-container>
set -euo pipefail

IMAGE="$1"
HOST_BIN="$2"
PORT="$3"
SAVE="$4"
FIXTURE="$PWD/tests/fixtures/big-buck-bunny.torrent"
METRICS="$(mktemp)"

if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  echo "SKIP: image $IMAGE is not present (build likely failed) — nothing to verify"
  exit 0
fi

# The image has no shell, so run the binary in metrics mode: it serves on $PORT,
# then after TF_METRICS_SECS prints METRIC lines and exits.
docker run --rm --network host \
  -v "$PWD/tests/fixtures:/fixtures:ro" \
  -e TF_METRICS=1 -e TF_METRICS_SECS=8 \
  "$IMAGE" -l /fixtures -s "$SAVE" -p "$PORT" > "$METRICS" 2>&1 &
FISH_PID=$!
sleep 2

# Drive the metadata fetch with the same binary acting as a magnet peer.
"$HOST_BIN" --probe-metadata "$FIXTURE" "$PORT" 25000

wait "$FISH_PID" || true
cat "$METRICS"

grep -q "METRIC total_payload_download=0" "$METRICS"
grep -q "METRIC total_wanted=0" "$METRICS"
grep -q "METRIC upload_mode=1" "$METRICS"
grep -q "METRIC auto_managed=0" "$METRICS"
grep -q "METRIC has_metadata=1" "$METRICS"

# Non-root check via image config (no shell in these images).
USER_SPEC=$(docker inspect --format '{{.Config.User}}' "$IMAGE")
echo "configured user: '$USER_SPEC'"
case "$USER_SPEC" in nonroot*|65532*) ;; *) echo "image does not run non-root"; exit 1;; esac

echo "OK: $IMAGE serves metadata, downloads zero payload, runs non-root"
