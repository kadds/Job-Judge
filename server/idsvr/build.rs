fn main() {
    tonic_build::configure()
        .build_server(true)
        .compile(&["proto/rpc.proto"], &["proto/", "../"])
        .unwrap();
}
