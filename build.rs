#![allow(warnings, unused)]

use bindgen::Bindings;
use cc;
use dunce;
use serde::Deserialize;
use std::collections::hash_map;
use std::ffi::OsStr;
use std::fmt::Display;
use std::fs::File;
use std::hash::Hasher;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::{env, fs, io};
macro_rules! log {
    ($($tokens: tt)*) => {
        let config_str =format!($($tokens)*);
        for line in config_str.lines() {
            println!("cargo:info={}", line);
        }

    }
}

const CONFIG_FILE: &str = "build.yaml";

#[derive(Debug, Deserialize)]
struct Config {
    pub lkh_home: String,
    pub lkh_obj_dir: Option<String>,
    pub lkh_src_dir: String,
    pub lkh_header_dir: String,
    pub lkh_wrapper_header: String,
    pub compile_bin: Option<String>,
    pub flags: Vec<String>,
}

impl Config {
    // path normalize
    fn normalize_paths(&mut self) {
        let cargo_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        log!("cargo_path:{:#?}", cargo_path);
        self.lkh_home = path_to_string(Path::new(&self.lkh_home), &cargo_path, true);
        self.lkh_obj_dir = match &self.lkh_obj_dir {
            Some(dir) => Some(path_to_string(Path::new(&dir), &cargo_path, true)),
            _ => Some(
                env::var_os("OUT_DIR")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            ),
        };
        self.lkh_src_dir = path_to_string(Path::new(&self.lkh_src_dir), &cargo_path, true);
        self.lkh_header_dir = path_to_string(Path::new(&self.lkh_header_dir), &cargo_path, true);
        self.lkh_wrapper_header =
            path_to_string(Path::new(&self.lkh_wrapper_header), &cargo_path, true);
    }

    fn get_sources(&self) -> Vec<String> {
        let mut sources = vec![];
        for entry in fs::read_dir(&self.lkh_src_dir).expect("read sources failed") {
            let path = entry.unwrap().path();
            if path.extension() == Some("c".as_ref()) {
                sources.push(path_to_string(
                    Path::new(&path),
                    &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
                    true,
                ));
            }
        }
        log!("sources:{:#?}", sources);
        sources
    }

    fn get_headers(&self) -> Vec<String> {
        let mut headers = vec![];
        for entry in fs::read_dir(&self.lkh_header_dir).expect("read headers failed") {
            let path = entry.unwrap().path();
            if path.extension() == Some("h".as_ref()) {
                headers.push(path_to_string(
                    Path::new(&path),
                    &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
                    true,
                ));
            }
        }
        log!("headers:{:#?}", headers);
        headers
    }
}

fn main() {
    // parse config from CONFIG_FILE
    let config = parse_config();
    // gen wrapper header file
    wrapper_header_build(&config);
    // compile obj
    compile_obj(&config);
    // bindgen config and link to rust
    generate_bindings(&config);
}

// parse config from yaml
fn parse_config() -> Config {
    // rebuild if config file change
    println!("cargo:rerun-if-changed={}", CONFIG_FILE);
    // parse config file
    let config_string = std::fs::read_to_string(CONFIG_FILE)
        .unwrap_or_else(|e| panic!("Unable to read {} file: {}", CONFIG_FILE, e));
    let mut config: Config = serde_yaml::from_str(&config_string)
        .unwrap_or_else(|e| panic!("Unable to parse {} file: {}", CONFIG_FILE, e));
    config.normalize_paths();
    // log config to terminal
    log!("LKH-BUILD configuration {} :: {:#?}", CONFIG_FILE, config);
    log!("LKH-BUILD headers {:#?}", config.get_headers());
    config
}

// path to relative path
fn path_to_string(path: &Path, base: &Path, to_unix: bool) -> String {
    if fs::metadata(path).is_err() {
        log!("path_to_string: path:{:?} base:{:#?}", path, base);
        fs::create_dir_all(path).unwrap();
    }
    let path1 = dunce::canonicalize(path).expect("path not found,can not canonicalize");
    let path2 = dunce::canonicalize(base).expect("path not found,can not canonicalize");
    let mut relative = Path::new(&path1)
        .strip_prefix(&path2)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    if to_unix {
        relative = relative.replace("\\", "/");
    }
    log!(
        "path1:{:?} to relative_path:{:?} with path2:{:#?}",
        path1,
        relative,
        path2
    );
    relative
}

// scan header and gen wrapper header
fn wrapper_header_build(config: &Config) {
    // collect all header file from header dir path
    let entries = fs::read_dir(&config.lkh_header_dir).expect("read header file dir failed");
    let header_paths: Vec<PathBuf> = entries
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.is_file() && path.extension() == Some("h".as_ref()) {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    // create wrapper file path
    let wrapper_file_path = Path::new(&config.lkh_wrapper_header);
    let mut file = match File::create(&wrapper_file_path) {
        Err(why) => panic!("couldn't create {}: {:?}", config.lkh_wrapper_header, why),
        Ok(file) => file,
    };

    // concat header file contents
    let mut contents = String::new();
    contents.push_str("/* automatically generated by LKH-rs */\n");
    for path in header_paths {
        let relative_path = path_to_string(&path, &wrapper_file_path.parent().unwrap(), true);
        contents.push_str(&format!("#include \"{}\"\n", relative_path));
    }

    // write content to wrapper file
    file.write_all(contents.as_bytes())
        .expect("write header file failed");
}

// compile obj
fn compile_obj(config: &Config) {
    let mut builder = cc::Build::new();
    // set flag
    for flag in &config.flags {
        builder.flag(flag);
    }
    // set other config
    builder
        .no_default_flags(true)
        .warnings(false)
        .extra_warnings(false)
        .out_dir(format!("{}/", &config.lkh_obj_dir.clone().unwrap()))
        .include(Path::new(&config.lkh_header_dir))
        .files(&config.get_sources());
    // set compile bin
    if let Some(bin) = &config.compile_bin {
        builder.compiler(bin);
    }
    // .compiler(Path::new(&config.compile_bin))
    log!("cc::Build::Config: {:?}", builder);
    builder.compile("lkh");
    // the cc:builder auto ptint cargo:rustc-link-lib in compile funtion
}

// bindgen config and generate
fn generate_bindings(config: &Config) {
    let bindings: Bindings = bindgen::Builder::default()
        .header(&config.lkh_wrapper_header)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");
    // Write the bindings to the CARGO_MANIFEST_DIR/src/bindings.rs file.
    let lkh_bind = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("bindings.rs");
    bindings
        .write_to_file(lkh_bind)
        .expect("Couldn't write bindings!");
}
