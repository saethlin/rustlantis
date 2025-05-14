#!/bin/bash

set -eu

while true; do 
    seed=$(python3 -c "print(int.from_bytes(open('/dev/urandom', 'rb').read(8), 'little'))")
    target/release/generate $seed > out/$seed.rs
    if target/release/difftest out/$seed.rs; then
        rm out/$seed.rs
    fi
done
