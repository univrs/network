//! Build script for mycelial-meshtastic
//!
//! This build script compiles Meshtastic protobufs using prost-build.
//! Proto files should be placed in the `proto/` directory.
//!
//! Currently, proto compilation is disabled until we vendor or submodule
//! the Meshtastic protobuf definitions. The generated types will be
//! added in a future phase.

fn main() {
    // Placeholder for prost-build configuration
    //
    // When proto files are added, uncomment and configure:
    //
    // let proto_files = &[
    //     "proto/meshtastic/mesh.proto",
    //     "proto/meshtastic/portnums.proto",
    //     "proto/meshtastic/config.proto",
    // ];
    //
    // let includes = &["proto"];
    //
    // prost_build::Config::new()
    //     .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
    //     .compile_protos(proto_files, includes)
    //     .expect("Failed to compile Meshtastic protobufs");

    // Trigger recompilation if build.rs changes
    println!("cargo:rerun-if-changed=build.rs");

    // Trigger recompilation if proto files change (future)
    println!("cargo:rerun-if-changed=proto/");
}
