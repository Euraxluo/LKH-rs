#![allow(warnings, unused)]

use bindgen::Bindings;
use cc;
use dunce;
use ignore;
use serde::Deserialize;
use std::collections::{hash_map, HashSet};
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
#[cfg(feature = "demo")]
#[cfg(unix)]
const CONFIG_FILE: &str = "build.demo.unix.yaml";

#[cfg(feature = "demo")]
#[cfg(windows)]
const CONFIG_FILE: &str = "build.demo.win.yaml";

#[cfg(not(feature = "demo"))]
#[cfg(windows)]
const CONFIG_FILE: &str = "build.win.yaml";

#[cfg(not(feature = "demo"))]
#[cfg(target_os = "macos")]
const CONFIG_FILE: &str = "build.osx.yaml";

#[cfg(not(feature = "demo"))]
#[cfg(not(target_os = "macos"))]
#[cfg(unix)]
const CONFIG_FILE: &str = "build.unix.yaml";

#[cfg(not(feature = "demo"))]
#[cfg(not(any(windows, unix)))]
const CONFIG_FILE: &str = "build.yaml";

#[derive(Debug)]
struct IgnoreMacros(HashSet<String>);

impl bindgen::callbacks::ParseCallbacks for IgnoreMacros {
    fn will_parse_macro(&self, name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        if self.0.contains(name) {
            bindgen::callbacks::MacroParsingBehavior::Ignore
        } else {
            bindgen::callbacks::MacroParsingBehavior::Default
        }
    }
}

#[derive(Debug, Deserialize)]
struct Config {
    pub lkh_home: String,
    pub lkh_obj_dir: Option<String>,
    pub lkh_src_dir: String,
    pub lkh_header_dir: String,
    pub lkh_wrapper_header: String,
    pub generate_wrapper_header: Option<bool>,
    pub compile_bin: Option<String>,
    pub flags: Vec<String>,
    pub excludes: Option<Vec<String>>,
}

impl Config {
    fn matches_excludes(&self, input: &str) -> bool {
        let ignore = match &self.excludes {
            Some(excludes_list) => {
                let mut build_ignore =
                    ignore::gitignore::GitignoreBuilder::new(env!("CARGO_MANIFEST_DIR"));
                for ele in excludes_list {
                    build_ignore.add_line(None, ele).expect("add_line failed");
                }
                build_ignore.build().expect("ignore_list build failed")
            }
            None => return false,
        };
        let path = Path::new(input);
        ignore
            .matched_path_or_any_parents(path, path.is_dir())
            .is_ignore()
    }
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
        self.generate_wrapper_header = match self.generate_wrapper_header {
            Some(flag) => Some(flag),
            _ => Some(true),
        };
    }

    fn get_sources(&self) -> Vec<String> {
        let mut sources = vec![];
        for entry in fs::read_dir(&self.lkh_src_dir).expect("read sources failed") {
            let path = entry.unwrap().path();
            if path.extension() == Some("c".as_ref()) {
                let path_str = path_to_string(
                    Path::new(&path),
                    &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
                    true,
                );
                if self.matches_excludes(&path_str) {
                    continue;
                }
                sources.push(path_str);
            }
        }
        sources
    }

    fn get_headers(&self) -> Vec<String> {
        let mut headers = vec![];
        for entry in fs::read_dir(&self.lkh_header_dir).expect("read headers failed") {
            let path = entry.unwrap().path();
            if path.extension() == Some("h".as_ref()) {
                let path_str = path_to_string(
                    Path::new(&path),
                    &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
                    true,
                );
                if self.matches_excludes(&path_str) {
                    continue;
                }
                headers.push(path_str);
            }
        }
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
    log!(
        "LKH-BUILD headers {:?} {:#?}",
        config.get_headers().len(),
        config.get_headers()
    );
    log!(
        "LKH-BUILD sources {:?} {:#?}",
        config.get_sources().len(),
        config.get_sources()
    );
    config
}

// path to relative path
fn path_to_string(path: &Path, base: &Path, to_unix: bool) -> String {
    if fs::metadata(path).is_err() {
        log!("path_to_string: path:{:?} base:{:#?}", path, base);
        if path.to_path_buf().extension() == None {
            fs::create_dir_all(path).unwrap();
        } else {
            fs::File::create(path).unwrap();
        }
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
    if !config.generate_wrapper_header.unwrap() {
        return;
    }
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
    let ignored_macros = IgnoreMacros(
        vec![
            "FP_INFINITE".into(),
            "FP_NAN".into(),
            "FP_NORMAL".into(),
            "FP_SUBNORMAL".into(),
            "FP_ZERO".into(),
        ]
        .into_iter()
        .collect(),
    );
    let bindings: Bindings = bindgen::Builder::default()
        .header(&config.lkh_wrapper_header)
        .parse_callbacks(Box::new(ignored_macros))
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
