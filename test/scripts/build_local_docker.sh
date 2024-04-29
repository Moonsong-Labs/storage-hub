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

if [ ! -f "target/release/storage-hub-node" ]; then
    echo "No node found, are you sure you've built it?"
    popd
    exit 1
fi

if ! file target/release/storage-hub-node | grep -q "x86-64"; then
    echo "The binary is not for x86 architecture."
    popd
    exit 1
fi

mkdir -p build

cp target/release/storage-hub-node build/

if ! docker build -t storage-hub:local -f docker/storage-hub-node.Dockerfile .; then
    echo "Docker build failed."
    popd
    exit 1
fi

popd

echo "Docker image built successfully."
