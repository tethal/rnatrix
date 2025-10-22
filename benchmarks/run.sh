#!/bin/bash

run_benchmark() {
    local benchmark="$1"
    echo "Running $benchmark..."
    cargo run --manifest-path ../Cargo.toml --bin natrix --release -- --ast harness.nx "$benchmark.nx" > "$benchmark.ast.txt"
    cargo run --manifest-path ../Cargo.toml --bin natrix --release -- --bc harness.nx "$benchmark.nx" > "$benchmark.bc.txt"
}

if [ -z "$1" ]; then
    # No argument - run all benchmarks
    for bench in *.nx; do
        # Skip harness.nx
        if [ "$bench" = "harness.nx" ]; then
            continue
        fi
        # Remove .nx extension
        benchmark="${bench%.nx}"
        run_benchmark "$benchmark"
    done
else
    # Run specific benchmark (remove .nx extension if present)
    benchmark="${1%.nx}"
    run_benchmark "$benchmark"
fi
