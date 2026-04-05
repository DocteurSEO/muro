#!/bin/bash
# Usage: ./run.sh [tiny|small|medium|large-v3-turbo]
export CXXFLAGS="-I/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/c++/v1"

# Charger les cles API depuis .env
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

export MURO_MODEL="${1:-tiny}"
cargo run --release
