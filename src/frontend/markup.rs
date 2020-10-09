use std::convert::AsRef;
use std::collections::VecDeque;
use std::path::{PathBuf, Path};
use structopt::StructOpt;
use serde::{Serialize, Deserialize};
use glob;
use kuchiki::traits::*;

use super::code;
use super::CompilerConfig;
use super::FileEntry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    source_path: PathBuf,
    contents: String,
}

pub fn strip_html_tags(value: &str) -> String {
    value
        .replace("<html>", "")
        .replace("</html>", "")
        .replace("<!DOCTYPE HTML>", "")
        .replace("<!DOCTYPE html>", "")
        .replace("<!doctype HTML>", "")
        .replace("<!doctype html>", "")
}

impl Template {
    pub fn new(config: &CompilerConfig) -> Self {
        let path = config.template_dir.join("index.html");
        let mut contents = String::from(include_str!(
            "../../assets/defaults/index.html"
        ));
        if path.exists() {
            contents = std::fs::read(&path)
                .ok()
                .and_then(|x| String::from_utf8(x).ok())
                .unwrap_or(contents);
        }
        Template {
            source_path: path,
            contents
        }
    }
    pub fn compile(
        &self,
        config: &CompilerConfig,
        path: &FileEntry
    ) -> String {
        compile_html_file(config, self, path)
    }
}



///////////////////////////////////////////////////////////////////////////////
// COMPILE HTML FILE
///////////////////////////////////////////////////////////////////////////////

pub fn compile_html_file(
    config: &CompilerConfig,
    template: &Template,
    path: &FileEntry,
) -> String {
    use pulldown_cmark::{html, Event, Options, Parser, Tag};
    use pulldown_cmark::CodeBlockKind;
    // SETUP
    let source = path.source.clone();
    // MARKDOWN API SETUP
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    // MARKDOWN CONVERSION
    let html_str = {
        let source = std::fs::read(&source).unwrap();
        let source = String::from_utf8(source).unwrap();
        // let source = format!("{}\n{}", toc, source);
        // CONVERT
        let parser = Parser::new_ext(&source, options);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);
        html_output
    };

    // TEMPLATE
    let index_file = template.contents.clone();

    let mut document = kuchiki::parse_html().one(index_file);

    // TOC
    {
        let source = ::markdown_toc::compile(&source);
        // CONVERT
        let parser = Parser::new_ext(&source, options);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);
        let html_output = format!(
            "<div toc>{}</div>",
            html_output
        );
        let source = fragment_to_html(Some("div"), &html_output);
        let source = strip_html_tags(&source.to_string());
        html_insert_str(None, "main", &mut document, &source);
    };
    
    // BODY
    html_insert_str(None, "main", &mut document, &html_str);

    // TRANSFORMS
    html_file_passes(path, &mut document);

    // DONE
    let doc = strip_html_tags(&document.to_string());
    format!("<!DOCTYPE html>\n<html>\n{}\n</html>", doc)
}

///////////////////////////////////////////////////////////////////////////////
// HTML PASSES
///////////////////////////////////////////////////////////////////////////////

/// Example:
/// - example/school-notes/source/calc/chapter1.md
/// - example/school-notes/source
/// Should return:
/// - calc/chapter1.md
pub fn strip_parent(path: &PathBuf, reference: &PathBuf) -> PathBuf {
    path.strip_prefix(reference).unwrap().to_owned()
}

pub fn html_file_passes(entry: &FileEntry, document: &mut kuchiki::NodeRef) {
    // TRANSFORMERS
    pub fn add_deps(entry: &FileEntry, document: &mut kuchiki::NodeRef) {
        let deps = include_str!("../../assets/deps.html");
        let css_defaults = format!(
            "<style>{}</style>",
            include_str!("../../assets/defaults.css")
        );
        html_insert_str(None, "body", document, deps);
        html_insert_str(None, "body", document, &css_defaults);
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
    pub fn links(entry: &FileEntry, document: &mut kuchiki::NodeRef) {
        fn slugify(text: &str) -> String {
            use percent_encoding::{percent_encode, CONTROLS};
            percent_encode(
                text.replace(" ", "-").to_lowercase().as_bytes(),
                CONTROLS,
            )
            .to_string()
        }
        let run = |node: kuchiki::NodeDataRef<kuchiki::ElementData>| -> Option<()> {
            use kuchiki::Attributes;
            let text_content = get_text_content(&node.as_node()).unwrap();
            let text_content = slugify(&text_content);
            // let text_content = format!("#{}", text_content);
            node.as_node().as_element().unwrap().attributes.borrow_mut().insert(
                String::from("id"),
                text_content,
            );
            // let new_node = callback(node.as_node())?;
            // node.as_node().insert_after(new_node);
            // node.as_node().detach();
            Some(())
        };
        for node in document.select("h1").unwrap().collect::<Vec<_>>() {
            let _ = run(node);
        }
        for node in document.select("h2").unwrap().collect::<Vec<_>>() {
            let _ = run(node);
        }
        for node in document.select("h3").unwrap().collect::<Vec<_>>() {
            let _ = run(node);
        }
        for node in document.select("h4").unwrap().collect::<Vec<_>>() {
            let _ = run(node);
        }
        for node in document.select("h5").unwrap().collect::<Vec<_>>() {
            let _ = run(node);
        }
        for node in document.select("h6").unwrap().collect::<Vec<_>>() {
            let _ = run(node);
        }
        for node in document.select("h7").unwrap().collect::<Vec<_>>() {
            let _ = run(node);
        }
    }
    // APPLY
    links(entry, document);
    exec_scripts(entry, document);
    add_deps(entry, document);
    latex(entry, document);
}


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



