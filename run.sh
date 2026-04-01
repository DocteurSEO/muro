#!/bin/bash
# Usage: ./run.sh [small|medium|large]
export CXXFLAGS="-I/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/c++/v1"
export GROQ_API_KEY="YOUR_GROQ_KEY"
export MURO_MODEL="${1:-small}"
cargo run --release
