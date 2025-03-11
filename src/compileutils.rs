use crate::procutils::*;
use crate::types::*;
use std::env;
use std::fs;
use std::path::Path;
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
            cd(entry.to_str().unwrap());

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
                            let mut path = cwd().into_path_buf();
                            path.push(y.as_str().unwrap());
                            acc.push(path.into_boxed_path());
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
                            let mut path = cwd().into_path_buf();
                            path.push(y.as_str().unwrap());
                            acc.push(path.into_boxed_path());
                            acc
                        })
                } else {
                    Vec::new()
                }
            };
            let mut headers = file.to_path_buf();
            headers.push(dep.get("headers").unwrap().as_str().unwrap());

            ret.push(BuildShared {
                root: entry.clone().into_boxed_path(),
                clean: dep.get("clean").unwrap().as_str().unwrap().to_string(),
                build: dep.get("build").unwrap().as_str().unwrap().to_string(),
                headers: headers.into_boxed_path(),
                objs,
                libs,
            });

            cd(prev.to_str().unwrap());
        }
    }

    ret
}
pub fn parse_toml(file: &Path) -> (BuildDirs, Vec<BuildShared>, Vec<BuildTarget>) {
    let prev = cwd();
    let parent = file.parent().unwrap();
    cd(parent.to_str().unwrap());
    // Extract I/O folders
    let toml = fs::read_to_string(file).unwrap();
    let toml = toml.parse::<Table>().unwrap();

    let build = canonicalize(toml.get("output").unwrap().as_str().unwrap());
    println!("Output Directory: {:?}", build);
    let deps = canonicalize(toml.get("dependencies").unwrap().as_str().unwrap());
    println!("Dependency Directory: {:?}", deps);
    let src = canonicalize(toml.get("sources").unwrap().as_str().unwrap());
    println!("Sources Directory: {:?}", src);

    assert!(build.exists());
    assert!(deps.exists());
    assert!(src.exists());

    // Extract dependencies
    let dependencies = parse_dependencies(&*deps);

    // Extract the host platform's build targets
    let targets = toml.get("Target").unwrap().as_table().unwrap();

    // (Name, Table)
    let host_targets: Vec<(&String, &Value)> = targets
        .iter()
        .map(|(name, target)| {
            if (env::consts::OS == "macos") {
                (name, target.get("osx").unwrap())
            } else if (env::consts::OS == "windows") {
                (name, target.get("win32").unwrap())
            } else if (env::consts::OS == "linux") {
                (name, target.get("linux").unwrap())
            } else {
                panic!("wtf");
            }
        })
        .collect();
    let host_targets: Vec<BuildTarget> = host_targets
        .iter()
        .map(|(name, toml)| {
            let compiler = toml.get("compiler").unwrap().as_str().unwrap();
            let linker = toml.get("linker").unwrap().as_str().unwrap();
            let interceptor = toml.get("interceptor").unwrap().as_str().unwrap();

            let entrypoint = toml.get("entrypoint").unwrap().as_str().unwrap();

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

            assert!(bin_exists(compiler));
            assert!(bin_exists(linker));
            assert!(bin_exists(interceptor));

            BuildTarget {
                compiler: compiler.to_string(),
                linker: linker.to_string(),
                interceptor: interceptor.to_string(),
                entrypoint: Path::new(entrypoint).into(),
                name: name.to_string(),
                compiler_args: compiler_args.clone(),
                linker_args: linker_args.clone(),
            }
        })
        .collect();

    cd(prev.to_str().unwrap());
    (
        BuildDirs {
            dependencies: deps,
            sources: src,
            output: build,
        },
        dependencies,
        host_targets,
    )
}

/// ! Returns the path of the output object file
pub fn compile(
    compiler: &str,
    source: &Box<Path>,
    build: &Box<Path>,
    args: &Vec<String>,
) -> Box<Path> {
    let mut output = build.clone().into_path_buf();
    let module = source.file_name().unwrap().to_str().unwrap();
    output.push(module);
    output.push(".o");
    execute(compiler, args, false, true);
    return output.into_boxed_path();
}

/// ! Recursively searches for files ending in .c/.cpp/c++/.mm
pub fn find_sources(path: &Path) -> Vec<Box<Path>> {
    let mut sources: Vec<Box<Path>> = Vec::new();

    for entry in WalkDir::new(path) {
        let path = entry.unwrap();
        let path = path.path();

        if path.ends_with(".c")
            || path.ends_with(".cpp")
            || path.ends_with(".c++")
            || path.ends_with(".mm")
        {
            sources.push(path.to_owned().into_boxed_path());
        }
    }

    sources
}

/// ! Recursively searches for files ending in .h/.hpp and generate
pub fn generate_include_paths(root: &str, suffix: &str) -> Vec<String> {
    let mut ret = Vec::new();
    let cwd = root.to_string();

    for entry in WalkDir::new(&cwd) {
        let entry = entry.unwrap();
        let absolute = entry.path().to_str().unwrap().to_string();

        if absolute.ends_with(suffix) {
            let via_root = absolute.replace(&cwd, "");
            let via_root = std::path::Path::new(&via_root);
            let mut dir = via_root.parent();
            loop {
                if dir == None {
                    break;
                }
                ret.push(dir.clone().unwrap().to_str().unwrap().to_string());
                dir = dir.unwrap().parent();
            }
        }
    }

    ret
}
