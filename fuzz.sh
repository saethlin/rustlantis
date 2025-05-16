#!/bin/bash
set -eu

cargo build --release

mkdir -p out
proc=$(nproc)
for job in $(seq 0 $((proc-1))); do
	nice -n 19 ./fuzz-job.sh &> $job.out &
done
wait
