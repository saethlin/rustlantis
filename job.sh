#!/bin/bash

set -eu

OUT=$1

while true
do 
    seed=$(python3 -c "print(int.from_bytes(open('/dev/urandom', 'rb').read(8), 'little'))")
    target/release/generate $seed > $OUT/$seed.rs
    if target/release/difftest $OUT/$seed.rs
    then
        rm $OUT/$seed.rs
    else
        mkdir -p out
        mv $OUT/$seed.rs out/
    fi
done
