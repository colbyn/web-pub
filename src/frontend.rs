use std::convert::AsRef;
use std::collections::VecDeque;
use std::path::{PathBuf, Path};
use structopt::StructOpt;
use serde::{Serialize, Deserialize};
use glob;
use kuchiki::traits::*;

pub mod markup;
use super::code;

pub fn source_default() -> PathBuf {PathBuf::from("source")}
pub fn template_default() -> PathBuf {PathBuf::from("template")}
pub fn output_default() -> PathBuf {PathBuf::from("output")}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebPubManifest {
    #[serde(default = "source_default")]
    source: PathBuf,
    #[serde(default = "template_default")]
    template: PathBuf,
    #[serde(default = "output_default")]
    output: PathBuf,
}

impl WebPubManifest {
    pub fn load_manifest_file<P: AsRef<Path>>(path: &P) -> Self {
        let path = path.as_ref().to_owned();
        let path = path.join("web-pub.toml");
        if !path.exists() {
            eprintln!("missing manifest file");
        }
        let contents = std::fs::read(path).expect(
            "path to the root project dir"
        );
        toml::de::from_slice::<WebPubManifest>(&contents).expect(
            "load manifest file"
        )
    }
}


#[derive(Serialize, Deserialize, Debug, StructOpt)]
pub enum WebPubCli {
    Build {
        #[structopt(short, long)]
        root: Option<PathBuf>
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerConfig {
    pub root_dir: PathBuf,
    pub source_dir: PathBuf,
    pub template_dir: PathBuf,
    pub output_dir: PathBuf,
}

pub fn main() {
    match WebPubCli::from_args() {
        WebPubCli::Build{root} => {
            let config = {
                let root_dir = root.unwrap_or(PathBuf::from("./"));
                let manifest = WebPubManifest::load_manifest_file(&root_dir);
                let config = CompilerConfig {
                    root_dir: root_dir.clone(),
                    source_dir: root_dir.join(manifest.source),
                    template_dir: root_dir.join(manifest.template),
                    output_dir: root_dir.join(manifest.output),
                };
                config
            };
            let source_glob = {
                format!(
                    "{}/**/*.md",
                    config.source_dir.to_str().unwrap()
                )
            };
            let template = markup::Template::new(&config);
            glob::glob(&source_glob)
                .unwrap()
                .filter_map(Result::ok)
                .map(|x| rebase_parent(
                    &x,
                    &config.source_dir,
                    &config.output_dir,
                ))
                .for_each(|path| {
                    let output_contents = template.compile(
                        &config,
                        &path
                    );
                    std::fs::create_dir_all(
                        path.output.parent().unwrap_or(&config.output_dir)
                    ).unwrap();
                    std::fs::write(
                        path.output_with_ext("html"),
                        output_contents
                    ).unwrap();
                });
        }
    }
}


///////////////////////////////////////////////////////////////////////////////
// HELPER TYPES
///////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub root: PathBuf,
    pub source: PathBuf,
    output: PathBuf,
}

impl FileEntry {
    pub fn output_with_ext(&self, ext: &str) -> PathBuf {
        let mut out = self.output.clone();
        out.set_extension(ext);
        out
    }
}


///////////////////////////////////////////////////////////////////////////////
// HELPER FUNCTIONS
///////////////////////////////////////////////////////////////////////////////

/// Example:
/// - example/school-notes/source/calc/chapter1.md
/// - example/school-notes/source
/// Should return:
/// - calc/chapter1.md
pub fn strip_parent(path: &PathBuf, reference: &PathBuf) -> PathBuf {
    path.strip_prefix(reference).unwrap().to_owned()
}

pub fn rebase_parent(
    path: &PathBuf,
    source_dir: &PathBuf,
    out_dir: &PathBuf
) -> FileEntry {
    let out = strip_parent(path, source_dir);
    let out = out_dir.join(path);
    let out = FileEntry {
        root: source_dir.clone(),
        source: path.clone(),
        output: out_dir.join(strip_parent(path, source_dir)),
    };
    out
}

