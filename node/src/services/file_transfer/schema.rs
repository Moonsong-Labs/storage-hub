//! Include sources generated from protobuf definitions.

pub(crate) mod v1 {
    pub(crate) mod provider {
        include!(concat!(env!("OUT_DIR"), "/api.v1.provider.rs"));
    }
}
