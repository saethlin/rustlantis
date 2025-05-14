#!/bin/bash
set -eu

cargo build --release

export RUST_LOG=info

mkdir -p out
proc=$(nproc)
for seed in $(seq 0 $((proc-1))); do
	./fuzz-job.sh &> $seed.out &
done
wait
