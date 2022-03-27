#!/bin/bash
set -e
tag='latest'

function build_and_push() {
    cd ./out/$1
    docker build -t ${DOCKER_PREFIX}$1:${tag} .
    docker push ${DOCKER_PREFIX}$1:${tag}
    cd ./../../
}
if [[ $1 = '-d' ]]; then
    tag=$(date +%s)
    shift
fi

if [ -z $1 ]; then
    build_and_push 'usersvr'
    build_and_push 'sessionsvr'
    build_and_push 'gateway'
    build_and_push 'idsvr'
    build_and_push 'testsvr'
    build_and_push 'containersvr'
    build_and_push 'sdwp'
else
    build_and_push $1
fi
