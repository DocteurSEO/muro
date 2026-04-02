#!/bin/bash
# Usage: ./run.sh [small|medium|large-v3-turbo]
export CXXFLAGS="-I/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/c++/v1"
export GROQ_API_KEYS="YOUR_GROQ_KEY,YOUR_GROQ_KEY,YOUR_GROQ_KEY,YOUR_GROQ_KEY"
export MURO_MODEL="${1:-tiny}"
cargo run --release
