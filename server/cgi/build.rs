use std::env;
use std::path::PathBuf;

fn main() {
    tonic_build::configure()
        .build_server(true)
        .compile(
            &["../proto/runner.proto"],
            &["../compilation/proto/", "../proto"],
        )
        .unwrap();
}
