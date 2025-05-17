#!/bin/bash

set -eu

export REPRODUCER=$1
export SYSROOT=$(cargo +nightly miri setup --print-sysroot)

cvise ./interesting.sh $REPRODUCER --timeout 5 --not-c
