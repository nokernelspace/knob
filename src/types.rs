use serde::Deserialize;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompileCommands(pub Vec<CompileCommand>);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompileCommand {
    pub directory: String,
    pub arguments: Vec<String>,
    pub file: String,
    pub output: String,
}

#[derive(Debug)]
pub struct BuildShared {
    pub root: Box<Path>,
    pub clean: String,
    pub build: String,
    pub headers: Box<Path>,
    pub objs: Vec<Box<Path>>,
    pub libs: Vec<Box<Path>>,
}

impl BuildShared {
    pub fn is_loose(&self) -> bool {
        self.libs.len() == 0
    }
}

#[derive(Debug)]
pub struct BuildDirs {
    pub dependencies: Box<Path>,
    pub sources: Box<Path>,
    pub output: Box<Path>,
}
#[derive(Debug)]
pub struct BuildTarget {
    pub compiler: String,
    pub linker: String,
    pub interceptor: String,
    pub entrypoint: Box<Path>,
    pub name: String,
    pub compiler_args: Vec<String>,
    pub linker_args: Vec<String>,
}
