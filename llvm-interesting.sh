#!/bin/bash

set -eu

REPRODUCER=$1

OUT=$(mktemp '/tmp/.rustlantis-XXXXXX')
function cleanup {
    rm $OUT
}
trap cleanup EXIT

NOPT=$(rustup run nightly miri $REPRODUCER --sysroot $SYSROOT -Zmiri-tree-borrows)

rustup run nightly rustc $REPRODUCER -Zmir-opt-level=0 -Copt-level=3 -o $OUT
OPT=$($OUT)

if [[ "$NOPT" == "$OPT" ]]
then
    exit 1
else
    echo "success"
fi
