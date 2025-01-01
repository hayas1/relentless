fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protos: Vec<_> = std::fs::read_dir("proto/")?
        .filter_map(|e| {
            let p = e.ok()?.path();
            (p.is_file() && matches!(p.extension()?.to_str()?, "proto")).then_some(p)
        })
        .collect();

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(std::path::PathBuf::from(std::env::var("OUT_DIR")?).join("file_descriptor.bin"))
        .compile_protos(&protos, &["proto/"])?;
    Ok(())
}
