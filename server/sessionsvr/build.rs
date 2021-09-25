fn main() {
    let descriptor_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("descriptor.bin");
    tonic_build::configure()
        .build_server(true)
        .file_descriptor_set_path(descriptor_path)
        .compile(&["proto/rpc.proto"], &["proto/", "../"])
        .unwrap();
}
