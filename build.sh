#!/bin/bash
set -e

dir="release"
flag="--release"
if [[ $DEBUG == "1" ]];then 
    dir="debug"
    flag=""
fi

function copy_data(){
    mkdir -p out/${1}/
    strip -s target/x86_64-unknown-linux-musl/${dir}/${1} -o out/${1}/${1}
    upx -5 out/${1}/${1}
    sed "s/\${{BINARY}}/${1}/g" server/Dockerfile.template > out/${1}/Dockerfile
}

function copy_sdwp(){
    mkdir -p out/sdwp/web/dist
    strip -s target/x86_64-unknown-linux-musl/${dir}/sdwp -o out/sdwp/sdwp
    upx -5 out/sdwp/sdwp
    sed "s/\${{BINARY}}/sdwp/g" server/Dockerfile.template > out/sdwp/Dockerfile
    cp -r server/sdwp/web/dist/* out/sdwp/web/dist/
}

function build_sdwp() {
    cargo build ${flag} --target x86_64-unknown-linux-musl --bin sdwp
    cd server/sdwp/web/
    yarn
    yarn build
    cd ../../../
}

function build() {
    if [ $1 = 'sdwp' ];then
        build_sdwp
        copy_sdwp
    else 
        cargo build ${flag} --target x86_64-unknown-linux-musl --bin ${1}
        copy_data $1
    fi
}

if [ -z $1 ]; then
    build 'usersvr'
    build 'sessionsvr'
    build 'gateway'
    build 'idsvr'
    build 'testsvr'
    build 'containersvr'
    build 'sdwp'
else
    build $1
fi
