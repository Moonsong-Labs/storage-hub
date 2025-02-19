//! Include sources generated from protobuf definitions.

pub mod v1 {
    pub mod provider {
        include!(concat!(env!("OUT_DIR"), "/api.v1.provider.rs"));
    }
}
