#!/bin/bash
set -e

function copy_data(){
    mkdir -p out/${1}/
    strip -s target/x86_64-unknown-linux-musl/release/${1} -o out/${1}/${1}
    upx -9 out/${1}/${1}
    sed "s/\${{BINARY}}/${1}/g" server/Dockerfile.template > out/${1}/Dockerfile
}

function copy_sdwp(){
    mkdir -p out/${1}/web/dist
    strip -s target/x86_64-unknown-linux-musl/release/${1} -o out/${1}/${1}
    upx -9 out/${1}/${1}
    sed "s/\${{BINARY}}/${1}/g" server/Dockerfile.template > out/${1}/Dockerfile
    cp -r server/sdwp/web/dist/* out/${1}/web/dist/
}

copy_data 'usersvr'
copy_data 'sessionsvr'
copy_data 'gateway'
copy_data 'idsvr'
copy_data 'testsvr'
copy_sdwp 'sdwp'