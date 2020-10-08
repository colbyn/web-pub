#![allow(unused)]

pub mod code;

use std::collections::VecDeque;
use std::path::PathBuf;
use structopt::StructOpt;
use serde::{Serialize, Deserialize};
use glob;
use kuchiki::traits::*;


///////////////////////////////////////////////////////////////////////////////
// HTML UTILS
///////////////////////////////////////////////////////////////////////////////

pub fn get_text_content(
    node: &kuchiki::NodeRef,
) -> Option<String> {
    if let Some(first_child) = node.first_child() {
        if let Some(text_ref) = first_child.as_text() {
            return Some(text_ref.borrow().clone())
        }
    }
    None
}

pub fn html_replace(
    selector: &str,
    document: &mut kuchiki::NodeRef,
    callback: impl Fn(&kuchiki::NodeRef) -> Option<kuchiki::NodeRef>,
) {
    let run = |node: kuchiki::NodeDataRef<kuchiki::ElementData>| -> Option<()> {
        let new_node = callback(node.as_node())?;
        node.as_node().insert_after(new_node);
        node.as_node().detach();
        Some(())
    };
    for node in document.select(selector).unwrap().collect::<Vec<_>>() {
        let _ = run(node);
    }
}


pub fn html_insert(
    selector: &str,
    document: &mut kuchiki::NodeRef,
    new_node: kuchiki::NodeRef,
) {
    for css_match in document.select(selector).unwrap() {
        let as_node = css_match.as_node();
        as_node.append(new_node.clone());
    }
}

pub fn html_insert_str(
    parent_tag: Option<&str>,
    selector: &str,
    document: &mut kuchiki::NodeRef,
    new_node: &str,
) {
    let fragment = fragment_to_html(parent_tag, new_node);
    html_insert(selector, document, fragment);
}

pub fn fragment_to_html(
    parent_tag: Option<&str>,
    value: &str
) -> kuchiki::NodeRef {
    let qual_name = html5ever::QualName::new(
        None,
        html5ever::Namespace::from("http://www.w3.org/1999/xhtml"),
        html5ever::LocalName::from(parent_tag.unwrap_or("div")),
    );
    let result = kuchiki::parse_fragment(qual_name,vec![]).one(value);
    if parent_tag.is_some() {
        return result.first_child().unwrap();
    }
    result
}

///////////////////////////////////////////////////////////////////////////////
// WEB-PUB UTILS
///////////////////////////////////////////////////////////////////////////////

pub fn find_file(input: &PathBuf, file: &PathBuf) -> Option<String> {
    let file = input.join(file);
    Some(String::from_utf8(std::fs::read(file).ok()?).ok()?)
}


///////////////////////////////////////////////////////////////////////////////
// WEB-PUB
///////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug, StructOpt)]
struct WebPub {
    #[structopt(short, long)]
    input: String,
    #[structopt(short, long)]
    output: String,
}

pub fn apply_transformer(entry: &FileEntry, document: &mut kuchiki::NodeRef) {
    // TRANSFORMERS
    pub fn add_deps(entry: &FileEntry, document: &mut kuchiki::NodeRef) {
        let deps = include_str!("../assets/deps.html");
        html_insert_str(None, "body", document, deps);
    }
    pub fn latex(entry: &FileEntry, document: &mut kuchiki::NodeRef) {
        html_replace("pre", document, |pre_node| {
            let code_node = pre_node.first_child()?;
            let text = get_text_content(&code_node)?;
            let new_node = fragment_to_html(None, &format!("<span code-block>$${}$$</span>", text));
            Some(new_node)
        });
        html_replace("code", document, |node| {
            let text = get_text_content(&node)?;
            let text = text.trim();
            if !(text.starts_with("$") && text.ends_with("$")) {
                return None;
            }
            let text = text.strip_prefix("$")?;
            let text = text.strip_suffix("$")?;
            let mut mark = String::new();
            let new_node = fragment_to_html(None, &format!("<span inline-code>\\({}\\)</span>", text));
            Some(new_node)
        });
    }
    pub fn exec_scripts(entry: &FileEntry, document: &mut kuchiki::NodeRef) {
        html_replace("script", document, |node| {
            let res = node.as_element()?
                .clone()
                .attributes
                .borrow()
                .get("run")
                .is_some();
            let src_path = node.as_element()?
                .clone()
                .attributes
                .borrow()
                .get("src")?
                .to_string();
            let src_path = PathBuf::from(src_path);
            let file_path = entry.source.parent().unwrap().join(src_path);
            let (module_path, javascript_code) = code::compile_code(
                &entry.root,
                &file_path
            );
            let new_node = fragment_to_html(None, &format!(
                "\n<div id=\"{id}\"></div><script>\n{file}\nthis.{module}.default(document.getElementById('{id}'))\n</script>\n",
                id=rand::random::<u64>(),
                module=module_path,
                file=javascript_code,

            ));
            Some(new_node)
        })
    }
    // APPLY
    exec_scripts(entry, document);
    add_deps(entry, document);
    latex(entry, document);
}


pub fn generate_html_for_markdown_file(
    input: &PathBuf,
    output: &PathBuf,
    index_files: &Vec<PathBuf>,
    path: &FileEntry
) -> String {
    use pulldown_cmark::{html, Event, Options, Parser, Tag};
    use pulldown_cmark::CodeBlockKind;
    // MARKDOWN API SETUP
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    // MARKDOWN CONVERSION
    let html_str = {
        let source = std::fs::read(&path.source).unwrap();
        let source = String::from_utf8(source).unwrap();
        let parser = Parser::new_ext(&source, options);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);
        html_output
    };

    // TEMPLATE
    let index_file_path = find_index_file_for(
        input,
        output,
        index_files,
        &path.source,
    ).unwrap();
    let index_file = std::fs::read(&index_file_path).unwrap();
    let index_file = String::from_utf8(index_file).unwrap();

    let mut document = kuchiki::parse_html().one(index_file);
    html_insert_str(None, "main", &mut document, &html_str);

    // TRANSFORMS
    apply_transformer(path, &mut document);

    // DONE
    let doc = document
        .to_string()
        .replace("<html>", "")
        .replace("</html>", "");
    format!("<!DOCTYPE html>\n<html>\n{}\n</html>", doc)
}


/// Example:
/// - example/school-notes/source/calc/chapter1.md
/// - example/school-notes/source
/// Should return:
/// - calc/chapter1.md
pub fn strip_parent(path: &PathBuf, reference: &PathBuf) -> PathBuf {
    path.strip_prefix(reference).unwrap().to_owned()
}

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

/// Example:
/// - [example/school-notes/source/calc/chapter1.md]
/// - example/school-notes/source
/// - example/school-notes/output
/// Should return:
/// - [example/school-notes/output/calc/chapter1.md]
pub fn rebase_parents(
    paths: &Vec<PathBuf>,
    paths_parent: &PathBuf,
    new_parent: &PathBuf
) -> Vec<FileEntry> {
    paths
        .iter()
        .map(|x| (
            x.clone(),
            strip_parent(x, paths_parent),
        ))
        .map(|x| (
            x.0.clone(),
            new_parent.join(x.1)
        ))
        .map(|(i, o)| FileEntry{
            root: paths_parent.clone(),
            source: i,
            output: o
        })
        .collect::<Vec<_>>()
}

pub fn process_index_files(
    input: &PathBuf,
    output: &PathBuf,
    files: &Vec<PathBuf>
) {
    let paths = rebase_parents(&files, &input, &output);
    for path in paths {
        std::fs::create_dir_all(path.output.parent().unwrap_or(&output));
    }
}

pub fn find_index_file_for(
    input: &PathBuf,
    output: &PathBuf,
    index_files: &Vec<PathBuf>,
    file: &PathBuf
) -> Option<PathBuf> {
    let paths = rebase_parents(&index_files, &input, &output);
    let file_parent = file.parent().unwrap_or(&input);
    for path in paths {
        let path_parent = path.source.parent().unwrap_or(&input);
        if path_parent == file_parent {
            return Some(path.source);
        }
    }
    // DEFAULT
    let root_index = input.join("index.html");
    if root_index.exists() {
        return Some(root_index);
    }
    // DONE
    None
}

pub fn process_markdown_files(
    input: &PathBuf,
    output: &PathBuf,
    index_files: &Vec<PathBuf>,
    files: Vec<PathBuf>
) {
    let paths = rebase_parents(&files, &input, &output);
    for path in paths {
        let html_str = generate_html_for_markdown_file(
            input,
            output,
            index_files,
            &path,
        );
        std::fs::create_dir_all(path.output.parent().unwrap_or(&output));
        std::fs::write(path.output_with_ext("html"), html_str);
    }
}


fn main() {
    let web_pub = WebPub::from_args();
    let css_files = glob::glob(&format!("{}/**/*.css", web_pub.input))
        .unwrap()
        .filter_map(|x| x.ok())
        .collect::<Vec<PathBuf>>();
    let js_files = glob::glob(&format!("{}/**/*.js", web_pub.input))
        .unwrap()
        .filter_map(|x| x.ok())
        .collect::<Vec<PathBuf>>();
    let ts_files = glob::glob(&format!("{}/**/*.ts", web_pub.input))
        .unwrap()
        .filter_map(|x| x.ok())
        .collect::<Vec<PathBuf>>();
    let index_files = glob::glob(&format!("{}/**/index.html", web_pub.input))
        .unwrap()
        .filter_map(|x| x.ok())
        .collect::<Vec<PathBuf>>();
    let html_files = glob::glob(&format!("{}/**/*.html", web_pub.input))
        .unwrap()
        .filter_map(|x| x.ok())
        .filter_map(|x| {
            if x.file_name()? == PathBuf::from("index.html") {
                None
            } else {
                Some(x)
            }
        })
        .collect::<Vec<PathBuf>>();
    let markdown_files = glob::glob(&format!("{}/**/*.md", web_pub.input))
        .unwrap()
        .filter_map(|x| x.ok())
        .collect::<Vec<PathBuf>>();
    let input = PathBuf::from(web_pub.input);
    let output = PathBuf::from(web_pub.output);
    process_index_files(&input, &output, &index_files);
    process_markdown_files(&input, &output, &index_files, markdown_files);
}
