use crate::compileutils::*;
use crate::procutils::*;
use crate::types::*;
use std::path::{Path, PathBuf};

/// ! Step 1 : Build the libraries and loose objects in `./.deps/`
/// ! Depends on Bash
pub fn build_shared(shared: &Vec<BuildShared>) {
    let mut loose_objs = Vec::new();
    // Build Shared Dependencies
    for dep in shared {
        let name = dep.root.file_name().unwrap().to_str().unwrap();

        let prev = cwd();
        cd(&dep.root.clone());
        execute(
            "bash",
            &vec!["-c".to_string(), dep.build.clone()],
            false,
            true,
        )
        .unwrap();
        cd(&prev.clone());

        // Archive loose dependencies as libdependencies.a
        if dep.is_loose() {
            println!("Built Loose Shared {}", name);
            loose_objs.append(&mut dep.objs.clone());
        } else {
            println!("Built Shared {}", name);
        }
    }

    // Archive loose dependencies as libdependencies.a
    if loose_objs.len() > 0 {
        println!("Archiving {} Loose Shared Objects...", loose_objs.len());
        let mut args = if std::env::consts::OS == "macos" {
            vec!["-r".to_string(), "libdependencies.a".to_string()]
        } else {
            vec!["r".to_string(), "libdependencies.a".to_string()]
        };
        args.append(
            &mut loose_objs
                .into_iter()
                .map(|x| (*x.to_str().unwrap()).to_string())
                .collect(),
        );
        execute(&"ar", &args, false, false).unwrap();
    }
}

/// ! Step 2
/// ! Compiles only the files in `./src/` and generates objects. Run with `bear` to generate `compile_commands.json`
pub fn compile_project(
    root: &PathBuf,
    platform: &BuildPlatform,
    shared: &Vec<BuildShared>,
    dirs: &BuildDirs,
) -> Vec<PathBuf> {
    // Compile Project Source
    let sources = find_sources(&dirs.sources);

    let mut compiler_args =
        generate_include_args(root, dirs, shared, &platform.compiler_args.clone());

    let mut objs = Vec::new();
    for source in sources {
        let o = compile(&platform.compiler, &source, &dirs.output, &compiler_args);
        objs.push(o);
    }

    objs
}

/// ! Step 3
/// !  Links a list of `*.o` into an executables. One of them must include `main()`. Suggested to
/// append `compile(entrypoint)` to the list of objects
pub fn link_binary(
    root: &PathBuf,
    platform: &BuildPlatform,
    shared: &Vec<BuildShared>,
    target: &BuildTarget,
    objs: &Vec<PathBuf>,
) {
    // Enter the directory of the Project root before linking
    let prev = cwd();
    cd(&root.clone());
    // Output binary path
    let mut binary = root.clone();
    binary.push(&target.name);

    let mut objs: Vec<String> = objs
        .iter()
        .map(|o| o.to_str().unwrap().to_string())
        .collect();

    let mut needs_to_link_libdeps = false;
    let mut libraries = Vec::new();
    for d in shared {
        if !d.is_loose() {
            libraries.append(&mut d.libs.clone());
        } else {
            needs_to_link_libdeps = true;
        }
    }
    println!(
        "Linking {} Modules => {}",
        objs.len() + libraries.len(),
        target.name.clone()
    );

    // Specify binary name
    let mut args = vec!["-o".to_string(), binary.to_str().unwrap().to_string()];
    // Generate -L paths for dependencies
    args.append(&mut generate_library_args(&libraries));
    // Link loose dependencies
    if needs_to_link_libdeps {
        args.push("-L".to_string() + root.to_str().unwrap());
        args.push("-ldependencies".to_string());
    }
    // Add Project.toml defined arguments
    args.append(&mut platform.linker_args.clone());
    args.append(&mut target.linker_args.clone());
    // Compiled project objects
    args.append(&mut objs);

    execute(&platform.linker, &args, false, true).unwrap();

    // Exit the project directory
    cd(&prev.clone());
}
