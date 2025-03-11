pub mod buildsteps;
pub mod compileutils;
pub mod procutils;
pub mod types;
use crate::compileutils::*;
use crate::procutils::*;
use crate::types::*;
use buildsteps::*;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// ! knob shared -> knob build <TARGET>
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = ".")]
    root: String,

    #[command(subcommand)]
    command: Option<Commands>,
}
#[derive(Subcommand)]
enum Commands {
    INIT,
    CLEAN,
    PROJECT { query: String },
    BUILD { query: String },
    RELEASE,
    INC { query: String },
    CHECK,
    SHARED,
}

fn main() {
    let args = Args::parse();
    let root = canonicalize(&args.root).into_path_buf();
    let mut toml = root.clone();
    toml.push("Project.toml");
    let mut compile_commands = root.clone();
    compile_commands.push("compile_commands.json");

    let mut db = root.clone();
    db.push("sources.db");

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
            Commands::CLEAN => {
                let (dirs, shared, targets) = parse_toml(&*toml);
                for dep in shared {
                    println!(
                        "Cleaning {}",
                        dep.root.file_name().unwrap().to_str().unwrap()
                    );
                    let prev = cwd();
                    cd(&*dep.root.to_str().unwrap());
                    execute(
                        "bash",
                        &vec!["-c".to_string(), dep.clean.clone()],
                        false,
                        false,
                    );
                    cd(&*prev.to_str().unwrap());
                }

                println!("Removing {:?}", dirs.output);
                rm(&*dirs.output);
                println!("Removing libdependencies.a");
                let mut libdep = root.clone();
                libdep.push("./libdependencies.a");
                let libdep = libdep.into_boxed_path();
                rm(&*libdep);

                println!("Removing compile_commands.json");
                rm(&*compile_commands.into_boxed_path());

                for target in targets {
                    println!("Removing {}", target.name);
                    let mut bin = root.clone();
                    bin.push(&target.name);
                    let bin = bin.into_boxed_path();
                    rm(&*bin);
                }
            }
            Commands::SHARED => {
                let (dirs, shared, targets) = parse_toml(&*toml);
                println!("Building Shared Dependencies...");
                build_shared(&shared);
            }
            Commands::PROJECT { query } => {
                let (dirs, shared, targets) = parse_toml(&*toml);

                let targets = targets.iter().fold(Vec::new(), |mut a, x| {
                    if query == "all" {
                        a.push(x)
                    } else if x.name.contains(&query) {
                        a.push(x)
                    }
                    a
                });

                let root = root.clone().into_boxed_path();

                println!("Linking {} Binaries", targets.len());
                for target in targets {
                    // TODO: Move compile_project() outside of this loop
                    // The problem is that there aren't cross-platform compiler & linker args in
                    // the root of Project.tomml
                    let objs = compile_project(&root, target, &shared, &dirs);
                    link_binary(&root, &shared, target, &objs);
                }
            }
            Commands::INC { query } => {
                let (dirs, shared, targets) = parse_toml(&*toml);
                let targets = targets.iter().fold(Vec::new(), |mut a, x| {
                    if query == "all" {
                        a.push(x);
                    } else if x.name.contains(&query) {
                        a.push(x)
                    }
                    a
                });
                let compile_commands =
                    std::fs::read_to_string(&*compile_commands.into_boxed_path()).unwrap();
                let compile_commands: CompileCommands =
                    serde_json::from_str(&compile_commands).unwrap();

                for target in targets {
                    let mut objs = Vec::new();
                    for command in compile_commands.0.clone() {
                        let bin = command.arguments[0].clone();
                        let args = command.arguments[1..].to_vec();
                        execute(&bin, &args, false, true);
                        objs.push(PathBuf::from(&command.output).into_boxed_path());
                    }

                    link_binary(&root.clone().into_boxed_path(), &shared, target, &objs);
                }
            }
            Commands::BUILD { query } => {
                let (dirs, shared, targets) = parse_toml(&*toml);

                let targets = targets.iter().fold(Vec::new(), |mut a, x| {
                    if query == "all" {
                        a.push(x);
                    } else if x.name.contains(&query) {
                        a.push(x)
                    }
                    a
                });
                let root = root.into_boxed_path();

                println!("Building Shared Depenencies...");
                build_shared(&shared);

                for target in targets {
                    println!("Compiling Project...");
                    let objs = compile_project(&root, target, &shared, &dirs);
                    println!("Linking Binary...");
                    link_binary(&root, &shared, target, &objs);
                }
            }
            Commands::RELEASE => {}
        },
        None => {
            panic!("Specify an action")
        }
    }
}
