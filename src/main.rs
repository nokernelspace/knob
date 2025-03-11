pub mod compileutils;
pub mod procutils;
pub mod types;
use crate::compileutils::*;
use crate::procutils::*;
use crate::types::*;
use clap::{Parser, Subcommand};
use std::path::Path;

/// Simple program to greet a person
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = ".")]
    root: String,

    #[arg(short, long, default_value = "")]
    target: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    INIT,
    CLEAN,
    BUILD,
    OPTS,
    RELEASE,
    INC,
    CHECK,
    SHARED, ///! Shared is actually a clone of BUILD but just the first part
}

fn main() {
    let args = Args::parse();
    let root = canonicalize(&args.root).into_path_buf();
    let mut toml = root.clone();
    toml.push("Project.toml");
    let toml = toml.into_boxed_path();
    println!("Config File: {:?}", toml);

    match args.command {
        Some(x) => match x {
            Commands::CHECK => {
                let (dirs, shared, targets) = parse_toml(&*toml);
                println!("Directories\n{:#?}", dirs);
                println!("Dependencies\n{:#?}", shared);
                println!("Targets\n{:#?}", targets);
            }
            Commands::INIT => {
                todo!()
            }
            Commands::CLEAN => {}
            Commands::BUILD => {
                let (dirs, shared, targets) = parse_toml(&*toml);

                let targets = targets.iter().fold(Vec::new(), |mut a, x| {
                    if x.name.contains(&args.target) {
                        a.push(x)
                    }
                    a
                });
                let mut loose_objs = Vec::new();
                // Build Shared Dependencies
                for dep in shared {
                    let name = dep.root.file_name().unwrap().to_str().unwrap();
                    println!("Building Shared {}", name);

                    let prev = cwd();
                    cd(&*dep.root.to_str().unwrap());
                    execute("bash", &vec!["-c".to_string(), dep.build], false, false);
                    cd(&*prev.to_str().unwrap());

                    // Archive loose dependencies as libdependencies.a
                    if dep.libs.len() == 0 {
                        loose_objs.append(&mut dep.objs.clone());
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

                // Compile Project Source
                // Compile Target Entrypoint
                // Link Entrypoint with
                //  - Project Sources      : generate_linker_args("./build")
                //  - libdependencies.a    : generate_library_args("libdependencies.a")
                //  - Library Dependencies : Provided via Dependency.toml and cli args
            }
            Commands::SHARED => {
                let (dirs, shared, targets) = parse_toml(&*toml);

                let targets = targets.iter().fold(Vec::new(), |mut a, x| {
                    if x.name.contains(&args.target) {
                        a.push(x)
                    }
                    a
                });
                let mut loose_objs = Vec::new();
                // Build Shared Dependencies
                for dep in shared {
                    let name = dep.root.file_name().unwrap().to_str().unwrap();
                    println!("Building Shared {}", name);

                    let prev = cwd();
                    cd(&*dep.root.to_str().unwrap());
                    execute("bash", &vec!["-c".to_string(), dep.build], false, false);
                    cd(&*prev.to_str().unwrap());

                    // Archive loose dependencies as libdependencies.a
                    if dep.libs.len() == 0 {
                        loose_objs.append(&mut dep.objs.clone());
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

                // Compile Project Source
                // Compile Target Entrypoint
                // Link Entrypoint with
                //  - Project Sources      : generate_linker_args("./build")
                //  - libdependencies.a    : generate_library_args("libdependencies.a")
                //  - Library Dependencies : Provided via Dependency.toml and cli args
            }
            Commands::OPTS => {}
            Commands::RELEASE => {}
            Commands::INC => {}
        },
        None => {
            panic!("Specify an action")
        }
    }
}
