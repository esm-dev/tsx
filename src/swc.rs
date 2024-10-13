use crate::dev::{DevFold, DevOptions};
use crate::error::{DiagnosticBuffer, ErrorBuffer};
use crate::import_analyzer::ImportAnalyzer;
use crate::resolver::Resolver;
use crate::swc_prefresh::swc_prefresh;

use base64::{engine::general_purpose, Engine as _};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use swc_common::comments::SingleThreadedComments;
use swc_common::errors::{Handler, HandlerFlags};
use swc_common::{chain, FileName, Globals, Mark, SourceMap};
use swc_ecma_transforms::optimization::simplify::dce;
use swc_ecma_transforms::pass::Optional;
use swc_ecma_transforms::proposals::decorators;
use swc_ecma_transforms::typescript::{tsx, typescript};
use swc_ecma_transforms::{fixer, helpers, hygiene, react};
use swc_ecmascript::ast::{EsVersion, Module, Program};
use swc_ecmascript::codegen::{text_writer::JsWriter, Config, Emitter, Node};
use swc_ecmascript::parser::{lexer, Parser};
use swc_ecmascript::parser::{EsSyntax, StringInput, Syntax, TsSyntax};
use swc_ecmascript::visit::{Fold, FoldWith};

/// Options for transpiling a module.
pub struct EmitOptions {
  pub source_map: Option<String>,
  pub dev: Option<DevOptions>,
  pub target: EsVersion,
  pub jsx_import_source: Option<String>,
  pub tree_shaking: bool,
}

impl Default for EmitOptions {
  fn default() -> Self {
    EmitOptions {
      source_map: None,
      dev: None,
      target: EsVersion::Es2022,
      jsx_import_source: None,
      tree_shaking: false,
    }
  }
}

#[derive(Clone)]
pub struct SWC {
  pub specifier: String,
  pub module: Module,
  pub source_map: Rc<SourceMap>,
  pub comments: SingleThreadedComments,
}

impl SWC {
  /// Parse the source code of a JS/TS module into an AST.
  pub fn parse(specifier: &str, source: &str) -> Result<Self, DiagnosticBuffer> {
    let source_map = SourceMap::default();
    let source_file = source_map.new_source_file(FileName::Real(Path::new(specifier).to_path_buf()).into(), source.into());
    let error_buffer = ErrorBuffer::new(specifier);
    let syntax = get_syntax(specifier);
    let input = StringInput::from(&*source_file);
    let comments = SingleThreadedComments::default();
    let lexer = lexer::Lexer::new(syntax, EsVersion::EsNext, input, Some(&comments));
    let mut parser = Parser::new_from(lexer);
    let handler = Handler::with_emitter_and_flags(
      Box::new(error_buffer.clone()),
      HandlerFlags {
        can_emit_warnings: true,
        dont_buffer_diagnostics: true,
        ..HandlerFlags::default()
      },
    );
    let sm = &source_map;
    let module = parser.parse_module().map_err(move |err| {
      let mut diagnostic = err.into_diagnostic(&handler);
      diagnostic.emit();
      DiagnosticBuffer::from_error_buffer(error_buffer, |span| sm.lookup_char_pos(span.lo))
    })?;

    Ok(SWC {
      specifier: specifier.into(),
      module,
      source_map: Rc::new(source_map),
      comments,
    })
  }

  /// Transpile a JS/TS module.
  pub fn transform(self, resolver: Rc<RefCell<Resolver>>, options: &EmitOptions) -> Result<(String, Option<String>), DiagnosticBuffer> {
    swc_common::GLOBALS.set(&Globals::new(), || {
      let unresolved_mark = Mark::new();
      let top_level_mark = Mark::new();
      let is_ts = self.specifier.ends_with(".ts") || self.specifier.ends_with(".tsx") || self.specifier.ends_with(".mts");
      let is_jsx = self.specifier.ends_with(".tsx") || self.specifier.ends_with(".jsx");
      let is_http_sepcifier = resolver.borrow().is_http_specifier;
      let is_dev = options.dev.is_some();
      let jsx_options = if let Some(jsx_import_source) = &options.jsx_import_source {
        react::Options {
          runtime: Some(react::Runtime::Automatic),
          import_source: Some(jsx_import_source.to_owned()),
          development: Some(is_dev),
          ..Default::default()
        }
      } else {
        react::Options {
          development: Some(is_dev),
          ..Default::default()
        }
      };
      let refresh_options = options.dev.clone().unwrap_or_default().refresh;
      let prefresh_options = options.dev.clone().unwrap_or_default().prefresh;
      let visitor = chain!(
        swc_ecma_transforms::resolver(unresolved_mark, top_level_mark, is_ts),
        // todo: support the new decorators proposal
        decorators::decorators(decorators::Config {
          legacy: true,
          emit_metadata: false,
          use_define_for_class_fields: false,
        }),
        Optional::new(typescript::strip(unresolved_mark, top_level_mark), is_ts && !is_jsx),
        Optional::new(
          tsx(
            self.source_map.clone(),
            typescript::Config::default(),
            typescript::TsxConfig {
              pragma: jsx_options.pragma.clone(),
              pragma_frag: jsx_options.pragma_frag.clone(),
            },
            Some(&self.comments),
            unresolved_mark,
            top_level_mark
          ),
          is_ts && is_jsx
        ),
        Optional::new(react::jsx_src(is_dev, self.source_map.clone()), is_jsx),
        Optional::new(react::jsx_self(is_dev), is_jsx),
        Optional::new(
          react::refresh(
            is_dev,
            Some(react::RefreshOptions {
              refresh_reg: "$RefreshReg$".into(),
              refresh_sig: "$RefreshSig$".into(),
              emit_full_signatures: false,
            }),
            self.source_map.clone(),
            Some(&self.comments),
            top_level_mark
          ),
          !is_http_sepcifier && (refresh_options.is_some() || prefresh_options.is_some())
        ),
        Optional::new(swc_prefresh(&self.specifier), !is_http_sepcifier && prefresh_options.is_some()),
        Optional::new(
          react::jsx(
            self.source_map.clone(),
            Some(&self.comments),
            jsx_options,
            top_level_mark,
            unresolved_mark,
          ),
          is_jsx
        ),
        Optional::new(react::display_name(), is_jsx),
        Optional::new(react::pure_annotations(Some(&self.comments)), is_jsx),
        fixer::paren_remover(Some(&self.comments)),
        ImportAnalyzer {
          resolver: resolver.clone(),
        },
        Optional::new(
          DevFold {
            specifier: self.specifier.clone(),
            options: options.dev.clone().unwrap_or_default(),
          },
          is_dev && !is_http_sepcifier
        ),
        helpers::inject_helpers(top_level_mark),
        Optional::new(dce::dce(Default::default(), unresolved_mark), options.tree_shaking),
        hygiene::hygiene_with_config(hygiene::Config {
          top_level_mark,
          ..Default::default()
        }),
        fixer(Some(&self.comments)),
      );

      // emit code
      let (mut code, map) = self.emit(visitor, options);

      // resolve jsx runtime path defined by `// @jsxImportSource` annotation
      let mut jsx_runtime = None;
      let resolver = resolver.borrow();
      for dep in &resolver.deps {
        if dep.specifier.ends_with("/jsx-runtime") || dep.specifier.ends_with("/jsx-dev-runtime") {
          jsx_runtime = Some((dep.specifier.clone(), dep.resolved_url.clone()));
          break;
        }
      }
      if let Some((jsx_runtime, import_url)) = jsx_runtime {
        code = code.replace(format!("\"{}\"", jsx_runtime).as_str(), format!("\"{}\"", import_url).as_str());
      }

      Ok((code, map))
    })
  }

  /// Emit code with a given set of visitor.
  fn emit<T: Fold>(&self, mut visitor: T, options: &EmitOptions) -> (String, Option<String>) {
    let eol = "\n";
    let program = Program::Module(self.module.clone());
    let program = helpers::HELPERS.set(&helpers::Helpers::new(false), || program.fold_with(&mut visitor));
    let mut js_buf = Vec::new();
    let mut map_buf = Vec::new();
    let writer = if options.source_map.is_some() {
      JsWriter::new(self.source_map.clone(), eol, &mut js_buf, Some(&mut map_buf))
    } else {
      JsWriter::new(self.source_map.clone(), eol, &mut js_buf, None)
    };
    let mut emitter = Emitter {
      cfg: Config::default().with_target(options.target).with_minify(false),
      comments: Some(&self.comments),
      cm: self.source_map.clone(),
      wr: writer,
    };
    program.emit_with(&mut emitter).expect("failed to emit code");

    let js = String::from_utf8(js_buf).expect("invalid utf8 character detected");
    if let Some(sm) = &options.source_map {
      let mut source_map = Vec::new();
      self
        .source_map
        .build_source_map_from(&mut map_buf, None)
        .to_writer(&mut source_map)
        .expect("failed to build source map");
      if sm.eq("inline") {
        let mut src = js;
        src.push_str("\n//# sourceMappingURL=data:application/json;base64,");
        src.push_str(&general_purpose::STANDARD.encode(source_map));
        (src, None)
      } else {
        (js, Some(String::from_utf8(source_map).expect("invalid utf8 character detected")))
      }
    } else {
      (js, None)
    }
  }
}

fn get_es_config(jsx: bool) -> EsSyntax {
  EsSyntax {
    fn_bind: true,
    export_default_from: true,
    allow_super_outside_method: true,
    allow_return_outside_function: true,
    decorators: true,
    jsx,
    ..EsSyntax::default()
  }
}

fn get_ts_config(tsx: bool) -> TsSyntax {
  TsSyntax {
    decorators: true,
    tsx,
    ..TsSyntax::default()
  }
}

fn get_syntax(specifier: &str) -> Syntax {
  let lang = specifier
    .split(|c| c == '?' || c == '#')
    .next()
    .unwrap()
    .split('.')
    .last()
    .unwrap_or("js")
    .to_lowercase();
  match lang.as_str() {
    "js" | "mjs" => Syntax::Es(get_es_config(false)),
    "jsx" => Syntax::Es(get_es_config(true)),
    "ts" | "mts" => Syntax::Typescript(get_ts_config(false)),
    "tsx" => Syntax::Typescript(get_ts_config(true)),
    _ => Syntax::Es(get_es_config(false)),
  }
}
