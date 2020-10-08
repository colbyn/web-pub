use std::convert::AsRef;
use std::{path::Path, sync::Arc};
use swc::{self, config::Options};
use swc_common::{
    errors::{ColorConfig, Handler},
    SourceMap,
};

// Works for TypeScript or JavaScript code based on file extension.
pub fn compile_code<P: AsRef<Path>>(file_path: &P) -> String {
    let cm = Arc::<SourceMap>::default();
    let handler = Arc::new(Handler::with_tty_emitter(
        ColorConfig::Auto,
        true,
        false,
        Some(cm.clone()),
    ));
    let c = swc::Compiler::new(cm.clone(), handler.clone());

    let fm = cm
        .load_file(file_path.as_ref())
        .expect("failed to load file");

    let out = c.process_js_file(
        fm,
        &Options {
            ..Default::default()
        },
    )
    .expect("failed to process file");
    out.code
}