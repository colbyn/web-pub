use std::path::PathBuf;
use std::convert::AsRef;
use std::{path::Path, sync::Arc};
use swc;
use swc::config::{self, Config, Options, ModuleConfig};
use swc_common::{
    errors::{ColorConfig, Handler},
    SourceMap,
};
use swc_ecma_transforms::modules;

// Works for TypeScript or JavaScript code based on file extension.
pub fn compile_code<P: AsRef<Path>>(
    root_dir: &P,
    file_path: &P
) -> (String, String) {
    let cm = Arc::<SourceMap>::default();
    let handler = Arc::new(Handler::with_tty_emitter(
        ColorConfig::Auto,
        true,
        false,
        Some(cm.clone()),
    ));
    let c = swc::Compiler::new(cm.clone(), handler.clone());

    let source = std::fs::read(&file_path);
    if let Err(err) = source {
        let file_path = file_path.as_ref().to_str().unwrap();
        eprintln!("source file not found: {}\n", file_path);
        panic!("failed to load file");
    }
    let source = source.unwrap();
    let source = String::from_utf8(source).expect("utf-8 source code");
    let module_name_hack = {
        let file_path = file_path.as_ref().to_owned();
        let file_path = file_path
            .strip_prefix(root_dir)
            .unwrap();
        let module_name = file_path
            .into_iter()
            .map(|x| x.to_str().unwrap().to_owned())
            .collect::<Vec<_>>()
            .join("");
        module_name
    };

    let fm = cm
        .new_source_file(
            swc_common::FileName::Real({
                PathBuf::from(module_name_hack.clone())
            }),
            source
        );

    let out = c.process_js_file(
        fm,
        &Options {
            // root: Some(PathBuf::from("example/school-notes/source/")),
            // filename: String::from("hello_world.js"),
            is_module: true,
            config: Some(Config {
                module: Some(ModuleConfig::Umd(
                    modules::umd::Config {
                        config: modules::util::Config{
                            strict: true,
                            strict_mode: true,
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                )),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .expect("failed to process file");
    let module_name_hack = module_name_hack
        .strip_suffix(".ts")
        .unwrap_or(&module_name_hack);
    let module_name_hack = module_name_hack
        .strip_suffix(".js")
        .unwrap_or(module_name_hack);
    (module_name_hack.to_owned(), out.code)
}