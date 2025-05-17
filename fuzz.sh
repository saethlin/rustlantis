#!/bin/bash
set -eu

cargo build --release

OUT=$(mktemp -d '/tmp/.rustlantis-XXXXXX')
function cleanup {
    kill $(jobs -p)
    rm -rf $OUT
}
trap cleanup EXIT

proc=16
for job in $(seq 0 $((proc-1))); do
	nice -n 19 ./job.sh "$OUT" &> $job.out &
done

wait
