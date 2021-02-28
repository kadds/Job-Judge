fn main() {
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile(
            &[
                "../proto/runner.proto",
                "../proto/user.proto",
                "../usersvr/proto/rpc.proto",
            ],
            &["../", "../usersvr/proto"],
        )
        .unwrap();
}
