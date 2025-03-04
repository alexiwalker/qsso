#!/bin/bash
cargo build --release
sudo cp ./target/release/qsso /usr/bin