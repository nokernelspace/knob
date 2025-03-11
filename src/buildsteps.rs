use crate::compileutils::*;
use crate::procutils::*;
use crate::types::*;
use std::path::Path;

pub fn link_binary(
    root: &Box<Path>,
    shared: &Vec<BuildShared>,
    target: &BuildTarget,
    objs: &Vec<Box<Path>>,
) {
    let prev = cwd();
    cd(&*root.to_str().unwrap());
    let mut binary = root.clone().into_path_buf();
    binary.push(&target.name);
    let binary = binary.into_boxed_path();
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
    args.append(&mut target.linker_args.clone());
    // Compiled project objects
    args.append(&mut objs);

    execute(&target.linker, &args, false, false);
    cd(&*prev.to_str().unwrap());
}

pub fn compile_project(
    root: &Box<Path>,
    target: &BuildTarget,
    shared: &Vec<BuildShared>,
    dirs: &BuildDirs,
) -> Vec<Box<Path>> {
    // Compile Project Source
    println!("Searching {:?}", &dirs.sources);
    let headers = find_headers(&dirs.sources);
    println!("Found {} Headers", headers.len());
    let sources = find_sources(&dirs.sources);
    println!("Found {} Sources", headers.len());
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

    let mut compiler_args = target.compiler_args.clone();
    let mut linker_args = target.compiler_args.clone();

    println!("{} Compiler Arguments", compiler_args.len());
    println!(
        "{} Include Arguments",
        includes_args.len() + isys_args.len()
    );

    compiler_args.append(&mut includes_args);
    compiler_args.append(&mut linker_args);

    let mut objs = Vec::new();
    for source in sources {
        let o = compile(&target.compiler, &source, &dirs.output, &compiler_args);
        objs.push(o);
    }

    // Compile Target Entrypoint
    let driver = compile(
        &target.compiler,
        &target.entrypoint,
        &dirs.output,
        &compiler_args,
    );

    objs.push(driver);
    objs
}

pub fn build_shared(shared: &Vec<BuildShared>) {
    let mut loose_objs = Vec::new();
    // Build Shared Dependencies
    for dep in shared {
        let name = dep.root.file_name().unwrap().to_str().unwrap();

        let prev = cwd();
        cd(&*dep.root.to_str().unwrap());
        execute(
            "bash",
            &vec!["-c".to_string(), dep.build.clone()],
            false,
            true,
        );
        cd(&*prev.to_str().unwrap());

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
