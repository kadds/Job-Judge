pub(crate) mod builtin {
    pub(crate) mod reflection {
        tonic::include_proto!("builtin.reflection");
        pub(crate) const FILE_DESCRIPTOR_SET: &'static [u8] =
            tonic::include_file_descriptor_set!("reflection_descriptor");
    }
}

pub mod client;
pub mod server;
