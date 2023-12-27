#!/usr/bin/env bash
set -ex

cd "$(dirname "$0")"

docker build -t chuilabs/rust .

read -r rustc version _ < <(docker run chuilabs/rust rustc --version)
[[ $rustc = rustc ]]
docker tag chuilabs/rust:latest chuilabs/rust:"$version"
docker push chuilabs/rust:"$version"
docker push chuilabs/rust:latest
