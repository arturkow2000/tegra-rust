#!/usr/bin/env bash
set -euo pipefail

prog="$1"
shift

ipv6=192.168.0.163

tmp="$(mktemp --tmpdir -d remote-run-XXXXXXXXX)"
master="$tmp/ssh.sock"
ssh_pid=""
remote_tmp=""
cleanup() {
    if [ -n "$remote_tmp" ]; then
        ssh -o BatchMode=yes -S "$master" root@$ipv6 "rm -rf '$remote_tmp'" || true
    fi

    if [ -n "$ssh_pid" ]; then
        kill $ssh_pid || true
        wait $ssh_pid || true
    fi
    rm -rf "$tmp"
}
trap cleanup EXIT

ssh -M -S "$master" -o ControlMaster=yes -o ControlPersist=no -o StrictHostKeyChecking=no root@$ipv6 -N &
ssh_pid=$!

remote_tmp="$(ssh -T -S "$master" root@$ipv6 "mktemp --tmpdir -d remote-run-XXXXXXXXX")"
remote_file="$remote_tmp/$(basename "$prog")"

status="$(ssh -T -S "$master" root@$ipv6 "cat > '$remote_file' && chmod +x '$remote_file'; echo $?" \
    < "$prog")"
if [ "$status" -ne 0 ]; then
    echo "upload failed: $status" >&2
    exit 1
fi

ssh -t -S "$master" root@$ipv6 "env RUST_LOG=debug RUST_BACKTRACE=1 SPDLOG_LEVEL=debug '$remote_file'; rm '$remote_file'"
