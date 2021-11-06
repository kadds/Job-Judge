#!/bin/bash
set -e

function build_and_push() {
    cd ./out/$1
    docker build -t ${DOCKER_PREFIX}$1:latest .
    docker push ${DOCKER_PREFIX}$1:latest
    cd ./../../
}

if [ -z $1 ]; then
    build_and_push 'usersvr'
    build_and_push 'sessionsvr'
    build_and_push 'gateway'
    build_and_push 'idsvr'
    build_and_push 'testsvr'
    build_and_push 'sdwp'
else
    build_and_push $1
fi
