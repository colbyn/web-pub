#![allow(unused)]

extern crate percent_encoding;

use std::fs::File;
use std::io::Read;
use percent_encoding::{percent_encode, CONTROLS};
use std::path::PathBuf;
use std::str::FromStr;

fn slugify(text: &str) -> String {
    percent_encode(
        text.replace(" ", "-").to_lowercase().as_bytes(),
        CONTROLS,
    )
    .to_string()
}

pub struct Heading {
    pub depth: usize,
    pub title: String,
}

impl FromStr for Heading {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim_end();
        if trimmed.starts_with("#") {
            let mut depth = 0usize;
            let title = trimmed
                .chars()
                .skip_while(|c| {
                    if *c == '#' {
                        depth += 1;
                        true
                    } else {
                        false
                    }
                })
                .collect::<String>()
                .trim_start()
                .to_owned();
            Ok(Heading {
                depth: depth - 1,
                title,
            })
        } else {
            Err(())
        }
    }
}

impl Heading {
    pub fn format(&self, config: &Config) -> Option<String> {
        if self.depth >= config.min_depth
            && config.max_depth.map(|d| self.depth <= d).unwrap_or(true)
        {
            Some(format!(
                "{}{} {}",
                " ".repeat(config.indent)
                    .repeat(self.depth - config.min_depth),
                &config.bullet,
                if config.no_link {
                    self.title.clone()
                } else {
                    format!("[{}](#{})", &self.title, slugify(&self.title))
                }
            ))
        } else {
            None
        }
    }
}

pub enum InputFile {
    Path(PathBuf),
    StdIn,
}

// enum Inline {
//     None,
//     Inline,
//     InlineAndReplace,
// }

pub struct Config {
    pub input_file: InputFile,
    pub bullet: String,
    pub indent: usize,
    pub max_depth: Option<usize>,
    pub min_depth: usize,
    pub header: Option<String>,
    pub no_link: bool,
    // inline: Inline,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            input_file: InputFile::StdIn,
            bullet: String::from("1."),
            indent: 4,
            max_depth: None,
            min_depth: 0,
            no_link: false,
            header: Some(String::from("## Table of Contents")),
            // inline: Inline::None,
        }
    }
}


pub fn compile(path: &PathBuf) -> String {
    let input_file = InputFile::Path(path.clone());
    let config = Config {
        input_file: input_file,
        bullet: String::from("1."),
        indent: 4,
        max_depth: None,
        min_depth: 0,
        // header: Some(String::from("## Table of Contents")),
        header: None,
        no_link: false,
    };

    let mut content = String::new();
    match config.input_file {
        InputFile::StdIn => std::io::stdin().read_to_string(&mut content),
        InputFile::Path(ref p) => File::open(p).unwrap().read_to_string(&mut content),
    }
    .unwrap();

    println!("");

    if let Some(ref header) = config.header {
        println!("{}\n", header);
    }

    let mut code_fence = Fence::None;

    content
        .lines()
        .filter(|line| match code_fence {
            Fence::None => {
                if line.starts_with("```") || line.starts_with("~~~") {
                    code_fence = Fence::Open(&line[..3]);
                    false
                } else {
                    true
                }
            }
            Fence::Open(tag) => {
                if line.starts_with(tag) {
                    code_fence = Fence::None;
                }
                false
            }
        })
        .map(Heading::from_str)
        .filter_map(Result::ok)
        .filter_map(|h| h.format(&config))
        .collect::<Vec<_>>()
        .join("\n")
}

enum Fence<'e> {
    Open(&'e str),
    None,
}
