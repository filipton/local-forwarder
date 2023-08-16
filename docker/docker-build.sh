#!/bin/bash

cwd=$(pwd)

cd ../client
cargo build -r
docker build -t filipton/lf-client -f .Dockerfile .
cd $cwd

cd ../server
cargo build -r
docker build -t filipton/lf-server -f .Dockerfile .
cd $cwd
