fn main() {
    let descriptor_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap())
        .join("reflection_descriptor.bin");
    tonic_build::configure()
        .file_descriptor_set_path(descriptor_path)
        .build_server(true)
        .build_client(true)
        .compile_well_known_types(true)
        .compile(&["proto/reflection.proto"], &["proto/"])
        .unwrap();
}
