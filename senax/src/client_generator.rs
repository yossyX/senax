use anyhow::Result;
use std::{fs, path::Path};

use crate::common::fs_write;

pub fn generate(name: &str, server: &str, _force: bool) -> Result<()> {
    anyhow::ensure!(Path::new("Cargo.toml").exists(), "Incorrect directory.");
    crate::common::check_ascii_name(name);

    for f in crate::TEMPLATES.file_names() {
        if f.starts_with("templates/client/") {
            let path = Path::new(name).join(f.trim_start_matches("templates/client/"));
            let buf = crate::TEMPLATES.get(f)?;
            if f.eq("templates/client/package.json") {
                let buf = std::str::from_utf8(buf.as_ref())?;
                let buf = buf.replace("<<client_name>>", name);
                fs_write(path, buf)?;
            } else {
                fs_write(path, buf)?;
            }
        }
    }

    let file_path = Path::new("./build.sh");
    if file_path.exists() {
        let content = fs::read_to_string(file_path)?.replace("\r\n", "\n");
        fs_write(file_path, fix_build_sh(&content, name, server)?)?;
    }

    Ok(())
}

fn fix_build_sh(content: &str, name: &str, server: &str) -> Result<String> {
    let content = content.replace(
        "# Do not modify this line. (Client)",
        &format!(
            "{}_client=\"--ts-dir {}\"\n# Do not modify this line. (Client)",
            server, name
        ),
    );
    let content = content.replace(
        "# Do not modify this line. (Codegen)",
        &format!(
            "codegen target/debug/{} {}\n# Do not modify this line. (Codegen)",
            server, name
        ),
    );
    Ok(content)
}
