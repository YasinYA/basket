#!/bin/sh
set -e

printf "Running cargo test before commit...\n"
cargo test
