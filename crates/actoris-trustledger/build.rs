fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Proto compilation is optional - we have manually created types in src/generated/
    // To enable auto-generation from proto files, uncomment below:
    //
    // tonic_build::configure()
    //     .build_server(true)
    //     .build_client(true)
    //     .out_dir("src/generated")
    //     .compile_protos(
    //         &[
    //             "../../proto/actoris/common.proto",
    //             "../../proto/actoris/trustledger.proto",
    //         ],
    //         &["../../proto"],
    //     )?;

    Ok(())
}
