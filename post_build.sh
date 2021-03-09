#!/bin/bash
function copy_data(){
    mkdir -p out/${1}/
    strip -s target/x86_64-unknown-linux-musl/release/${1} -o out/${1}/${1}
    upx -9 out/${1}/${1}
    sed "s/\${{BINARY}}/${1}/g" server/Dockerfile.template > out/${1}/Dockerfile
}
copy_data 'usersvr'
copy_data 'sdwp'
copy_data 'sessionsvr'
copy_data 'gateway'