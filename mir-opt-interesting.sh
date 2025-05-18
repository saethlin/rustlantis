#!/bin/bash

set -eu

rustup run nightly miri $REPRODUCER --sysroot $MIRI_SYSROOT -Zmiri-tree-borrows

if rustup run nightly miri $REPRODUCER --sysroot $MIRI_SYSROOT -Zmiri-tree-borrows -Zmir-opt-level=4
then
    exit 1
else
    echo "success"
fi
