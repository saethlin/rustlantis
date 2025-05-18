#!/bin/bash
set -eu

cargo build --release
export MIRI_SYSROOT=$(cargo +nightly miri setup --print-sysroot)

function cleanup {
    kill $(jobs -p)
}
trap cleanup EXIT

proc=16
for job in $(seq 0 $((proc-1))); do
	nice -n 19 ./job.sh &> $job.out &
done

wait
