use includedir_codegen::Compression;

fn main() {
    println!("cargo:rerun-if-changed=templates");

    includedir_codegen::start("TEMPLATES")
        .dir("templates", Compression::Gzip)
        .build("templates.rs")
        .unwrap();
}
