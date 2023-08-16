#!/bin/bash

cwd=$(pwd)
echo "Enter the version number: "
read version

cd ../client
cargo build -r
docker build -t filipton/lf-client -f .Dockerfile .
cd $cwd

cd ../server
cargo build -r
docker build -t filipton/lf-server -f .Dockerfile .
cd $cwd

docker tag filipton/lf-client filipton/lf-client:$version
docker tag filipton/lf-server filipton/lf-server:$version

docker push filipton/lf-client:$version
docker push filipton/lf-server:$version

docker tag filipton/lf-client filipton/lf-client:latest
docker tag filipton/lf-server filipton/lf-server:latest

docker push filipton/lf-client:latest
docker push filipton/lf-server:latest

