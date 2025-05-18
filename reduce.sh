#!/bin/bash

set -eu

cvise ./target/release/difftest $1 --not-c --shaddap --timeout 5
