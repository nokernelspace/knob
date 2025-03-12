use serde::Deserialize;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompileCommands(pub Vec<CompileCommand>);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompileCommand {
    pub directory: String,
    pub arguments: Vec<String>,
    pub file: String,
    pub output: String,
}

#[derive(Debug, Clone)]
pub struct BuildShared {
    pub root: PathBuf,
    pub clean: String,
    pub build: String,
    pub headers: PathBuf,
    pub objs: Vec<PathBuf>,
    pub libs: Vec<PathBuf>,
}

impl BuildShared {
    pub fn is_loose(&self) -> bool {
        self.libs.len() == 0
    }
}
#[derive(Debug, Clone)]
pub struct BuildPlatform {
    pub compiler: String,
    pub linker: String,
    pub compiler_args: Vec<String>,
    pub linker_args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BuildDirs {
    pub dependencies: PathBuf,
    pub sources: PathBuf,
    pub output: PathBuf,
}
#[derive(Debug, Clone)]
pub struct BuildTarget {
    pub entrypoint: PathBuf,
    pub name: String,
    pub compiler_args: Vec<String>,
    pub linker_args: Vec<String>,
}
