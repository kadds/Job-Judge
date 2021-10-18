function build() {
    cargo build --release --target x86_64-unknown-linux-musl --bin ${1}
}

function build_sdwp() {
    build $1
    cd server/sdwp/web/
    yarn
    yarn build
}

build 'usersvr'
build 'sessionsvr'
build 'gateway'
build 'idsvr'
build 'testsvr'
build 'sdwp'
build_sdwp 'sdwp'