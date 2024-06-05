#!/bin/bash

# ==============================================================================
# Script Name: build_and_deploy.sh
# Description: Build a Storage Hub Docker Image by copying the binary to a
#              build directory, checking for correct architecture, and then
#              packaging it.
# Usage: ./build_and_deploy.sh
# Requirements: This script must be run in a '/test' directory.
#
# Note: The script checks for for architecture compatibility of the
#       binary as we only support x86 architecture for now.
# ==============================================================================

export DOCKER_DEFAULT_PLATFORM=linux/amd64

pushd ../

if ! cargo install --list | grep -q cargo-zigbuild; then
    cargo install cargo-zigbuild --locked
fi

if ! rustup target list --installed | grep -q x86_64-unknown-linux-gnu; then
    rustup target add x86_64-unknown-linux-gnu
fi

cargo zigbuild --target x86_64-unknown-linux-gnu --release

if [ ! -f "target/x86_64-unknown-linux-gnu/release/storage-hub-node" ]; then
    echo "No node found, something must have gone wrong."
    popd
    exit 1
fi

if ! file target/x86_64-unknown-linux-gnu/release/storage-hub-node | grep -q "x86-64"; then
    echo "The binary is not for x86 architecture, something must have gone wrong."
    popd
    exit 1
fi

mkdir -p build

cp target/x86_64-unknown-linux-gnu/release/storage-hub-node build/

if ! docker build -t storage-hub:local -f docker/storage-hub-node.Dockerfile --load .; then
    echo "Docker build failed."
    popd
    exit 1
fi

popd

echo "Docker image built successfully."