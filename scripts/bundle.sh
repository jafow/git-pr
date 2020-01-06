#!/usr/bin/env bash

# tag the repo
if [[ -z $(which semtag) ]]; then
  printf "no valid version of semtag found in PATH\n"
  exit 1
fi

tag=$(semtag alpha -s patch -o)
name="git-pr-${tag}-x86_64-unknown-linux-gnu"

mkdir -p "/tmp/$name/"

cp ./target/release/git-pr "/tmp/$name/"
cp ./README.md "/tmp/$name/"
cp ./LICENSE-* "/tmp/$name/"

tar -czf "./builds/$name.tar.gz" -C /tmp "${name}"
