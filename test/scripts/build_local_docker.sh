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

ARCH=$(uname -m)
OS=$(uname -s)

## BUILD NODE BINARY

if [[ "$ARCH" == "arm64" && "$OS" == "Darwin" ]]; then
    BINARY_PATH="target/x86_64-unknown-linux-gnu/release/storage-hub-node"
else
    BINARY_PATH="target/release/storage-hub-node"
fi

## CHECK BINARY

if [ ! -f "$BINARY_PATH" ]; then
    echo "No node found, something must have gone wrong."
    popd
    exit 1
fi

if ! file "$BINARY_PATH" | grep -q "x86-64"; then
    echo "The binary is not for x86 architecture, something must have gone wrong."
    popd
    exit 1
fi

## BUILD DOCKER IMAGE
mkdir -p build

cp "$BINARY_PATH" build/

if ! docker build -t storage-hub:local -f docker/storage-hub-node.Dockerfile --load .; then
    echo "Docker build failed."
    popd
    exit 1
fi

popd

echo "Docker image built successfully."