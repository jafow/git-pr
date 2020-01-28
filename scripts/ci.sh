#!/usr/bin/env sh

set -ex

TARGET=x86_64-apple-darwin

rustup target add $TARGET

export MACOS_DEPLOYMENT_TARGET=10.7

cargo build --release --target $TARGET 
