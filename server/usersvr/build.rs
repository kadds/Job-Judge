fn main() {
	tonic_build::configure()
		.build_server(true)
		.type_attribute(".table", "#[derive(::sqlx::FromRow)]")
		.compile(&["proto/rpc.proto", "proto/table.proto"], &["proto/", "../"])
		.unwrap();
}
