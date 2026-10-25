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

# Upload a debug-stripped copy of the binary. A build-std debug binary is ~37M,
# almost all DWARF, which dominates the transfer over the device's slow link.
# --strip-debug drops it to ~6.5M while keeping the symbol table, so panic
# backtraces still name functions. Drop --strip-debug for a full strip (~2.3M)
# if you don't need them. The real artifact is left untouched (-o writes a copy).
strip_tool=armv7l-unknown-linux-gnueabihf-strip
upload="$prog"
local_tmp=""
if command -v "$strip_tool" >/dev/null 2>&1; then
    local_tmp="$(mktemp --tmpdir remote-run-XXXXXXXX)"
    if "$strip_tool" --strip-debug -o "$local_tmp" "$prog" 2>/dev/null; then
        upload="$local_tmp"
    fi
fi

remote_tmp=""
cleanup() {
    if [ -n "$remote_tmp" ]; then
        ssh "${ssh_opts[@]}" root@$ipv6 "rm -rf '$remote_tmp'" || true
    fi
    if [ -n "$local_tmp" ]; then
        rm -f "$local_tmp"
    fi
}
trap cleanup EXIT

# The first ssh here auto-starts the persistent master (ControlMaster=auto).
remote_tmp="$(ssh "${ssh_opts[@]}" root@$ipv6 "mktemp --tmpdir -d remote-run-XXXXXXXXX")"
remote_file="$remote_tmp/$(basename "$prog")"

ssh "${ssh_opts[@]}" root@$ipv6 "cat > '$remote_file' && chmod +x '$remote_file'" < "$upload"

ssh -t "${ssh_opts[@]}" root@$ipv6 "env RUST_LOG=debug RUST_BACKTRACE=1 SPDLOG_LEVEL=debug '$remote_file'"
