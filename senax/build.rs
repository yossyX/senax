use includedir_codegen::Compression;
use std::io::Write;
use std::process::{exit, Command};
use std::{env, io};

fn main() {
    println!("cargo:rerun-if-changed=templates");
    includedir_codegen::start("TEMPLATES")
        .dir("templates", Compression::Gzip)
        .build("templates.rs")
        .unwrap();

    if cfg!(feature = "config") {
        if Ok("release".to_owned()) == env::var("PROFILE") {
            let output = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .arg("/C")
                    .current_dir(std::env::current_dir().unwrap().join("config-app"))
                    .arg("npm install && npm run build")
                    .output()
                    .expect("failed to execute process")
            } else {
                Command::new("bash")
                    .arg("-c")
                    .current_dir(std::env::current_dir().unwrap().join("config-app"))
                    .arg("npm install && npm run build")
                    .output()
                    .expect("failed to execute process")
            };
            if !output.status.success() {
                io::stderr().write_all(&output.stderr).unwrap();
                exit(1);
            }
        }
        println!("cargo:rerun-if-changed=config-app/dist");
        includedir_codegen::start("CONFIG_APP")
            .dir("config-app/dist", Compression::Gzip)
            .build("config_app.rs")
            .unwrap();
    }
}
