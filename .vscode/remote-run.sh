#!/usr/bin/env bash
set -euo pipefail

prog="$1"
shift

ipv6=192.168.0.163

# Keep the SSH connection open in the background between runs. A master
# connection is opened on first use at a stable socket path and reused by later
# invocations; ControlPersist leaves it running for $persist after the last
# client disconnects, so repeated `cargo run`/`cargo test` don't pay the TCP +
# key-exchange + auth cost every time.
persist=10m
master="${XDG_RUNTIME_DIR:-/tmp}/remote-run-${ipv6}.sock"
ssh_opts=(
    -o ControlMaster=auto
    -o ControlPath="$master"
    -o ControlPersist="$persist"
    -o StrictHostKeyChecking=no
)

remote_tmp=""
cleanup() {
    if [ -n "$remote_tmp" ]; then
        ssh "${ssh_opts[@]}" root@$ipv6 "rm -rf '$remote_tmp'" || true
    fi
}
trap cleanup EXIT

# The first ssh here auto-starts the persistent master (ControlMaster=auto).
remote_tmp="$(ssh "${ssh_opts[@]}" root@$ipv6 "mktemp --tmpdir -d remote-run-XXXXXXXXX")"
remote_file="$remote_tmp/$(basename "$prog")"

ssh "${ssh_opts[@]}" root@$ipv6 "cat > '$remote_file' && chmod +x '$remote_file'" < "$prog"

ssh -t "${ssh_opts[@]}" root@$ipv6 "env RUST_LOG=debug RUST_BACKTRACE=1 SPDLOG_LEVEL=debug '$remote_file'"
