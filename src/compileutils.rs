use crate::procutils::*;
use crate::types::*;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Table;
use toml::Value;
use walkdir::WalkDir;

/// ! Parse the .deps folder
pub fn parse_dependencies(file: &Path) -> Vec<BuildShared> {
    let mut ret = Vec::new();
    for entry in WalkDir::new(file).max_depth(1) {
        if let Ok(entry) = &entry {
            let entry = entry.clone().into_path();

            // Skip over the self reference, for some reason it is included in walkdir
            if entry.clone().into_boxed_path() == file.into() {
                continue;
            }

            // Change directory into the current dependency folder
            let prev = cwd();
            cd(&entry.clone());

            let mut dep = entry.clone();
            dep.push("Dependency.toml");
            let dep = dep.as_path();
            let dep = fs::read_to_string(dep).unwrap();
            let dep = dep.parse::<Table>().unwrap();

            let libs = {
                if let Some(libs) = dep.get("Libraries") {
                    libs.as_table()
                        .unwrap()
                        .iter()
                        .fold(Vec::new(), |mut acc, (x, y)| {
                            let mut path = cwd();
                            path.push(y.as_str().unwrap());
                            acc.push(path);
                            acc
                        })
                } else {
                    Vec::new()
                }
            };
            let objs = {
                if let Some(objs) = dep.get("Objects") {
                    objs.as_table()
                        .unwrap()
                        .iter()
                        .fold(Vec::new(), |mut acc, (x, y)| {
                            let mut path = cwd();
                            path.push(y.as_str().unwrap());
                            acc.push(path);
                            acc
                        })
                } else {
                    Vec::new()
                }
            };
            let mut headers = entry.clone().to_path_buf();
            headers.push(dep.get("headers").unwrap().as_str().unwrap());

            ret.push(BuildShared {
                root: entry.clone(),
                clean: dep.get("clean").unwrap().as_str().unwrap().to_string(),
                build: dep.get("build").unwrap().as_str().unwrap().to_string(),
                headers: headers.clone(),
                objs,
                libs,
            });

            cd(&prev.clone());
        }
    }

    ret
}
pub fn parse_toml(file: &Path) -> (BuildDirs, BuildPlatform, Vec<BuildShared>, BuildTarget) {
    let prev = cwd();
    let parent = file.parent().unwrap();
    cd(&parent.to_path_buf());
    let toml = fs::read_to_string(file).unwrap();
    let toml = toml.parse::<Table>().unwrap();

    // Extract I/O folders
    let output = toml.get("output").unwrap().as_str().unwrap();
    let dependencies = toml.get("dependencies").unwrap().as_str().unwrap();
    let sources = toml.get("sources").unwrap().as_str().unwrap();

    // Make Directory if it doesn't Exist
    mkdir(&PathBuf::from(output));
    mkdir(&PathBuf::from(dependencies));
    mkdir(&PathBuf::from(sources));

    // Format to absolute paths
    let build = canonicalize(output);
    println!("Output Directory: {:?}", build);
    let deps = canonicalize(dependencies);
    println!("Dependency Directory: {:?}", deps);
    let src = canonicalize(sources);
    println!("Sources Directory: {:?}", src);

    // Select Platform Depending on Host
    let platforms = toml.get("Platform").unwrap().as_table().unwrap();
    let platform = {
        if env::consts::OS == "macos" {
            platforms.get("osx").unwrap().as_table().unwrap()
        } else if env::consts::OS == "windows" {
            platforms.get("win32").unwrap().as_table().unwrap()
        } else if env::consts::OS == "linux" {
            platforms.get("linux").unwrap().as_table().unwrap()
        } else {
            panic!("wtf");
        }
    };

    let platform = BuildPlatform {
        compiler: platform
            .get("compiler")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
        linker: platform
            .get("linker")
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
        compiler_args: platform
            .get("compiler_args")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect(),
        linker_args: platform
            .get("linker_args")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect(),
    };

    // Extract dependencies
    let dependencies = parse_dependencies(&*deps);

    // Extract the host platform's build targets
    let targets = toml.get("Target").unwrap().as_table().unwrap();

    // Parse Project

    // Parse Targets
    let targets: Vec<BuildTarget> = targets
        .iter()
        .map(|(name, toml)| {
            let entrypoint = toml.get("entrypoint").unwrap().as_str().unwrap();
            let entrypoint = canonicalize(entrypoint);

            let compiler_args = toml
                .get("compiler_args")
                .unwrap()
                .as_array()
                .unwrap()
                .into_iter()
                .map(|x| x.as_str().unwrap().to_string())
                .collect::<Vec<String>>();
            let linker_args = toml
                .get("linker_args")
                .unwrap()
                .as_array()
                .unwrap()
                .into_iter()
                .map(|x| x.as_str().unwrap().to_string())
                .collect::<Vec<String>>();

            let dependencies = toml
                .get("deps")
                .unwrap()
                .as_array()
                .unwrap()
                .into_iter()
                .map(|x| x.as_str().unwrap().to_string())
                .collect();

            BuildTarget {
                entrypoint,
                dependencies,
                name: name.to_string(),
                compiler_args: compiler_args.clone(),
                linker_args: linker_args.clone(),
            }
        })
        .collect();

    if targets.len() > 1 {
        panic!("Only 1 Build Target is Supported at a Time");
    }

    cd(&prev.clone());
    (
        BuildDirs {
            dependencies: deps,
            sources: src,
            output: build,
        },
        platform,
        dependencies,
        targets[0].clone(),
    )
}

/// ! Returns the path of the output object file
pub fn compile(compiler: &str, source: &PathBuf, build: &PathBuf, args: &Vec<String>) -> PathBuf {
    let mut output = build.clone();
    println!(
        "Compiling {}...",
        source.file_name().unwrap().to_str().unwrap()
    );
    let module = source.to_str().unwrap().to_string().replace("/", ".") + ".o";
    output.push(module);
    let mut _args = vec!["-c".to_string(), source.to_str().unwrap().to_string()];
    _args.append(&mut args.clone());
    _args.append(&mut vec![
        "-o".to_string(),
        output.to_str().unwrap().to_string(),
    ]);

    execute(compiler, &_args, true, true).unwrap();
    return output;
}

/// ! Recursively searches for files ending in .c/.cpp/c++/.mm
pub fn find_sources(path: &Path) -> Vec<PathBuf> {
    let mut sources: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(path) {
        let path = entry.unwrap();
        let path = path.path().to_path_buf();

        match path.extension() {
            Some(x) => {
                if x == ("c") || x == ("cpp") || x == ("c++") || x == "mm" {
                    sources.push(path.clone());
                }
            }
            _ => {}
        }
    }

    sources
}
/// ! Recursively searches for files ending in .h/.hpp/.h++
pub fn find_headers(path: &PathBuf) -> Vec<PathBuf> {
    let mut sources: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(path) {
        let path = entry.unwrap();
        let path = path.path().to_path_buf();

        match path.extension() {
            Some(x) => {
                if x == ("h") || x == ("hpp") || x == ("h++") {
                    sources.push(path.clone());
                }
            }
            _ => {}
        }
    }

    sources
}

/// ! Recursively searches for files ending in .h/.hpp/.gpp and generate
/// ! Example:
/// ! For /User/test/game/src/engine/api/header.h
/// ! - /User/test/game/src
/// ! - /User/test/game/src/engine
/// ! - /User/tes/game/src/engine/api
/// ! We return a HashSet to remove duplicates
pub fn generate_include_paths(root: &Path, headers: Vec<PathBuf>) -> HashSet<PathBuf> {
    let mut ret = HashSet::new();

    for header in headers {
        let mut iter = header.parent().unwrap();
        while iter != root {
            ret.insert(iter.to_path_buf());
            iter = iter.parent().unwrap();
        }
    }

    ret
}

/// ! Given a list of library files genreate the -L argument for linking
/// ! Keep in mind the library still must be specified per target with -lsdl3 in the Project.toml
pub fn generate_library_args(libs: &Vec<PathBuf>) -> Vec<String> {
    let mut args = Vec::new();
    for l in libs {
        let arg = "-L".to_string() + l.parent().unwrap().to_str().unwrap();
        args.push(arg);
    }
    args
}

pub fn generate_include_args(
    root: &PathBuf,
    dirs: &BuildDirs,
    shared: &Vec<BuildShared>,
    compiler_args: &Vec<String>,
) -> Vec<String> {
    let headers = find_headers(&dirs.sources);
    let mut includes = generate_include_paths(&root, headers);
    // Add the shared dependency includes to the list
    for dep in shared {
        includes.insert(dep.headers.clone());
    }

    let mut includes_args: Vec<String> = includes
        .iter()
        .map(|i| "-I".to_string() + i.to_str().unwrap())
        .collect();
    let mut isys_args = includes.iter().fold(Vec::new(), |mut a, i| {
        a.push("-isystem".to_string());
        a.push(i.to_str().unwrap().to_string());
        a
    });

    let mut compiler_args = compiler_args.clone();
    compiler_args.append(&mut includes_args);
    compiler_args.append(&mut isys_args);

    compiler_args
}
