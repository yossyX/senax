use includedir_codegen::Compression;
use walkdir::WalkDir;

fn main() {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=seeds");

    let mut codegen = includedir_codegen::start("SEEDS");
    for entry in WalkDir::new("seeds").follow_links(true).into_iter() {
        match entry {
            Ok(ref e) if !e.file_type().is_dir() => {
                let comp = match e.path().extension() {
                    Some(ext) if ext == "yml" => Compression::Gzip,
                    _ => Compression::None,
                };
                codegen.add_file(e.path(), comp).unwrap();
            }
            _ => (),
        }
    }
    codegen.build("seeds.rs").unwrap();
}
@{-"\n"}@