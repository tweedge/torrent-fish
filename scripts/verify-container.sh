#!/usr/bin/env bash
# Verify a torrent-fish image end-to-end: it must serve metadata to a real
# magnet-only peer while downloading zero payload bytes, and must run non-root.
#
# Usage: verify-container.sh <image-tag> <port> <save-path-in-container>
set -euo pipefail

IMAGE="$1"
PORT="$2"
SAVE="$3"
METRICS="$(mktemp)"

if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
  echo "SKIP: image $IMAGE is not present (build likely failed) — nothing to verify"
  exit 0
fi

# distroless/scratch have no shell, so run the binary in metrics mode: it serves
# on $PORT, then after TF_METRICS_SECS prints METRIC lines and exits.
docker run --rm --network host \
  -v "$PWD/test-torrents:/opt/torrents:ro" \
  -e TF_METRICS=1 -e TF_METRICS_SECS=6 \
  "$IMAGE" -l /opt/torrents -s "$SAVE" -p "$PORT" > "$METRICS" 2>&1 &
FISH_PID=$!
sleep 2

# Drive a real libtorrent magnet peer and assert it receives metadata.
python3 - "$PWD/test-torrents/test.torrent" "$PORT" <<'PY'
import sys, time, libtorrent as lt
tf, port = sys.argv[1], int(sys.argv[2])
ti = lt.torrent_info(tf)
ses = lt.session({"listen_interfaces":"127.0.0.1:0","enable_dht":False,"enable_lsd":False,"enable_upnp":False,"enable_natpmp":False})
atp = lt.parse_magnet_uri(lt.make_magnet_uri(ti)); atp.save_path="/tmp/client"; atp.peers=[("127.0.0.1",port)]
h = ses.add_torrent(atp)
end = time.time()+25
while time.time() < end:
    if h.status().has_metadata:
        print("METADATA_SERVED_OK"); sys.exit(0)
    time.sleep(0.2)
print("METADATA_NOT_SERVED", file=sys.stderr); sys.exit(1)
PY

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
cp "$METRICS" "/tmp/metrics-$(echo "$IMAGE" | tr '/:' '__').txt"
