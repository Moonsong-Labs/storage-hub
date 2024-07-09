const PROTOS: &[&str] = &["src/schema/provider.v1.proto"];

fn main() {
    // Tell Cargo to rerun this build script whenever the proto files change.
    PROTOS.iter().for_each(|proto| {
        println!("cargo:rerun-if-changed={}", proto);
    });

    prost_build::compile_protos(PROTOS, &["src/services/file_transfer/schema"]).unwrap();
}
