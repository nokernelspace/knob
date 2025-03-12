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
    PROJECT,
    BUILD { query: String },
    RELEASE,
    INC { query: String },
    CHECK,
    SHARED,
}

fn main() {
    let args = Args::parse();
    let root = canonicalize(&args.root);
    let mut toml = root.clone();
    toml.push("Project.toml");
    let mut compile_commands = root.clone();
    compile_commands.push("compile_commands.json");

    let toml = toml.into_boxed_path();
    println!("Config File: {:?}", toml);

    match args.command {
        Some(x) => match x {
            Commands::CHECK => {
                let (dirs, platform, shared, targets) = parse_toml(&*toml);
                println!("Directories\n{:#?}", dirs);
                println!("Dependencies\n{:#?}", shared);
                println!("Targets\n{:#?}", targets);
            }
            Commands::INIT => {
                todo!()
            }
            Commands::CLEAN => {
                let (dirs, _, shared, targets) = parse_toml(&*toml);
                for dep in shared {
                    println!(
                        "Cleaning {}",
                        dep.root.file_name().unwrap().to_str().unwrap()
                    );
                    let prev = cwd();
                    cd(&dep.root.clone());
                    execute(
                        "bash",
                        &vec!["-c".to_string(), dep.clean.clone()],
                        false,
                        false,
                    );
                    cd(&prev.clone());
                }

                println!("Removing {:?}", dirs.output);
                rm(&dirs.output);
                println!("Removing libdependencies.a");
                let mut libdep = root.clone();
                libdep.push("./libdependencies.a");
                rm(&libdep);

                println!("Removing compile_commands.json");
                rm(&compile_commands);

                for target in targets {
                    println!("Removing {}", target.name);
                    let mut bin = root.clone();
                    bin.push(&target.name);
                    rm(&bin);
                }
            }
            Commands::SHARED => {
                let (dirs, platform, shared, targets) = parse_toml(&*toml);
                println!("Building Shared Dependencies...");
                build_shared(&shared);
            }
            Commands::PROJECT => {
                let (dirs, platform, shared, targets) = parse_toml(&*toml);

                compile_project(&root, &platform, &shared, &dirs);
            }
            Commands::INC { query } => {
                let (dirs, platform, shared, targets) = parse_toml(&*toml);
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

                let headers = find_headers(&dirs.sources);
                let sources = find_sources(&dirs.sources);
                let mut includes = generate_include_paths(&root, headers);
                // Add the shared dependency includes to the list
                for dep in shared.clone() {
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

                for target in targets {
                    let mut compiler_args = platform.compiler_args.clone();
                    compiler_args.append(&mut includes_args.clone());
                    compiler_args.append(&mut isys_args.clone());
                    let mut objs = Vec::new();
                    for command in compile_commands.0.clone() {
                        let bin = command.arguments[0].clone();
                        let args = command.arguments[1..].to_vec();
                        let src = PathBuf::from(&command.file);
                        let obj = PathBuf::from(&command.output);
                        objs.push(obj);

                        let o = last_modified(&command.output);
                        let c = last_modified(&command.file);

                        if o.is_err() || c.is_err() || o.unwrap() < c.unwrap() {
                            println!("Rebuilding {}", command.output);
                            execute(&bin, &args, false, true);
                        }
                    }

                    let bin = compile(
                        &platform.compiler,
                        &target.entrypoint,
                        &dirs.output,
                        &compiler_args,
                    );
                    objs.push(bin.clone());

                    link_binary(&root.clone(), &platform, &shared, target, &objs);
                }
            }
            Commands::BUILD { query } => {
                let (dirs, platform, shared, targets) = parse_toml(&*toml);

                let targets = targets.iter().fold(Vec::new(), |mut a, x| {
                    if query == "all" {
                        a.push(x);
                    } else if x.name.contains(&query) {
                        a.push(x)
                    }
                    a
                });

                println!("Building Shared Depenencies...");
                build_shared(&shared);

                for target in targets {
                    println!("Compiling Project...");
                    let mut objs = compile_project(&root, &platform, &shared, &dirs);

                    // Compile the entrypoint
                    let mut args = generate_include_args(
                        &root,
                        &dirs,
                        &shared,
                        &platform.compiler_args.clone(),
                    );

                    println!("Compiling {}...", target.name);
                    let entrypoint =
                        compile(&platform.compiler, &target.entrypoint, &dirs.output, &args);
                    objs.push(entrypoint);
                    println!("Linking Binary...");
                    link_binary(&root, &platform, &shared, target, &objs);
                }
            }
            Commands::RELEASE => {}
        },
        None => {
            panic!("Specify an action")
        }
    }
}
