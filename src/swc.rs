use crate::dev::{Dev, DevOptions};
use crate::error::{DiagnosticBuffer, ErrorBuffer};
use crate::import_analyzer::ImportAnalyzer;
use crate::resolver::Resolver;
use crate::specifier::is_http_specifier;
use crate::swc_jsx_src::jsx_source;
use crate::swc_prefresh::swc_prefresh;
use base64::{engine::general_purpose, Engine as _};
use std::cell::RefCell;
use std::fmt;
use std::path::Path;
use std::rc::Rc;
use swc_atoms::Atom;
use swc_common::comments::SingleThreadedComments;
use swc_common::errors::{Handler, HandlerFlags};
use swc_common::pass::Optional;
use swc_common::source_map::{SourceMap, SourceMapGenConfig};
use swc_common::{FileName, Globals, Mark};
use swc_ecma_transforms::optimization::simplify::dce;
use swc_ecma_transforms::proposals::decorators;
use swc_ecma_transforms::typescript::{tsx, typescript};
use swc_ecma_transforms::{fixer, helpers, hygiene, react};
use swc_ecmascript::ast::{EsVersion, Module, Pass, Program};
use swc_ecmascript::codegen::{text_writer::JsWriter, Config, Emitter, Node};
use swc_ecmascript::parser::{lexer, Parser};
use swc_ecmascript::parser::{EsSyntax, StringInput, Syntax, TsSyntax};
use swc_ecmascript::visit::fold_pass;

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

#[derive(Debug)]
pub struct EmitError {
  pub message: String,
}

impl fmt::Display for EmitError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&self.message)
  }
}

pub struct SourceMapGenOptions {}

impl SourceMapGenConfig for SourceMapGenOptions {
  fn file_name_to_source(&self, f: &FileName) -> String {
    f.to_string()
  }
  fn inline_sources_content(&self, f: &FileName) -> bool {
    f.is_real()
  }
}

pub struct SWC {
  syntax: Syntax,
  module: Module,
  comments: SingleThreadedComments,
  source_map: Rc<SourceMap>,
}

impl SWC {
  /// Parse a module from a string.
  pub fn parse(specifier: &str, source: &str, lang: Option<String>) -> Result<Self, DiagnosticBuffer> {
    let syntax = get_syntax(specifier, lang);
    let source_map = SourceMap::default();
    let source_file = source_map.new_source_file(FileName::Real(Path::new(specifier).to_path_buf()).into(), source.into());
    let input = StringInput::from(&*source_file);
    let comments = SingleThreadedComments::default();
    let lexer = lexer::Lexer::new(syntax, EsVersion::EsNext, input, Some(&comments));
    let error_buffer = ErrorBuffer::new(specifier);
    let handler = Handler::with_emitter_and_flags(
      Box::new(error_buffer.clone()),
      HandlerFlags {
        can_emit_warnings: true,
        dont_buffer_diagnostics: true,
        ..HandlerFlags::default()
      },
    );
    let sm = &source_map;
    let module = Parser::new_from(lexer).parse_module().map_err(move |err| {
      let mut diagnostic = err.into_diagnostic(&handler);
      diagnostic.emit();
      DiagnosticBuffer::from_error_buffer(error_buffer, |span| sm.lookup_char_pos(span.lo))
    })?;

    Ok(SWC {
      syntax,
      module,
      comments,
      source_map: Rc::new(source_map),
    })
  }

  /// Transform the module to JavaScript and optionally generate a source map.
  pub fn transform(self, resolver: Rc<RefCell<Resolver>>, options: &EmitOptions) -> Result<(String, Option<String>), EmitError> {
    swc_common::GLOBALS.set(&Globals::new(), || {
      let pass = self.build_pass(resolver.clone(), options);
      let (mut code, map) = self.emit(pass, options)?;

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

  fn build_pass<'a>(&'a self, resolver: Rc<RefCell<Resolver>>, options: &EmitOptions) -> impl Pass + 'a {
    let top_level_mark = Mark::new();
    let unresolved_mark = Mark::new();
    let specifier = resolver.borrow().specifier.clone();
    let is_ts = if let Syntax::Typescript(ts) = self.syntax { !ts.tsx } else { false };
    let is_tsx = if let Syntax::Typescript(ts) = self.syntax { ts.tsx } else { false };
    let is_jsx = if let Syntax::Es(es) = self.syntax { es.jsx } else { false };
    let is_http_sepcifier = is_http_specifier(&specifier);
    let is_dev = options.dev.is_some();
    let dev_options = options.dev.clone().unwrap_or_default();
    let jsx_options = if let Some(jsx_import_source) = &options.jsx_import_source {
      react::Options {
        runtime: Some(react::Runtime::Automatic),
        import_source: Some(Atom::from(jsx_import_source.as_str())),
        development: Some(is_dev),
        ..Default::default()
      }
    } else {
      react::Options {
        development: Some(is_dev),
        ..Default::default()
      }
    };

    // https://github.com/swc-project/swc/pull/9680
    (
      swc_ecma_transforms::resolver(unresolved_mark, top_level_mark, is_ts),
      // todo: support the new decorators proposal
      decorators::decorators(decorators::Config {
        legacy: true,
        emit_metadata: false,
        use_define_for_class_fields: false,
      }),
      Optional::new(typescript::strip(unresolved_mark, top_level_mark), is_ts),
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
          top_level_mark,
        ),
        is_tsx,
      ),
      // jsx features passes
      Optional::new(
        (
          Optional::new(
            jsx_source(
              self.source_map.clone(),
              dev_options.jsx_source.as_ref().map(|opts| opts.file_name.clone()),
            ),
            dev_options.jsx_source.is_some(),
          ),
          react::jsx_self(is_dev),
          react::jsx(
            self.source_map.clone(),
            Some(&self.comments),
            jsx_options,
            top_level_mark,
            unresolved_mark,
          ),
          react::display_name(),
          react::pure_annotations(Some(&self.comments)),
        ),
        is_jsx || is_tsx,
      ),
      // analyze imports
      fold_pass(ImportAnalyzer {
        resolver: resolver.clone(),
      }),
      // dev mode
      Optional::new(
        (
          Optional::new(
            react::refresh(
              true,
              Some(react::RefreshOptions {
                refresh_reg: "$RefreshReg$".into(),
                refresh_sig: "$RefreshSig$".into(),
                emit_full_signatures: false,
              }),
              self.source_map.clone(),
              Some(&self.comments),
              top_level_mark,
            ),
            dev_options.refresh.is_some() || dev_options.prefresh.is_some(),
          ),
          Optional::new(swc_prefresh(&specifier), dev_options.prefresh.is_some()),
          fold_pass(Dev {
            resolver: resolver.clone(),
            options: options.dev.clone().unwrap_or_default(),
          }),
        ),
        is_dev && !is_http_sepcifier,
      ),
      // optimization passes
      (
        fixer::paren_remover(Some(&self.comments)),
        helpers::inject_helpers(top_level_mark),
        Optional::new(dce::dce(Default::default(), unresolved_mark), options.tree_shaking),
        hygiene::hygiene_with_config(hygiene::Config {
          top_level_mark,
          ..Default::default()
        }),
        fixer(Some(&self.comments)),
      ),
    )
  }

  fn emit<P: Pass>(&self, pass: P, options: &EmitOptions) -> Result<(String, Option<String>), EmitError> {
    let program = Program::Module(self.module.clone());
    let program = helpers::HELPERS.set(&helpers::Helpers::new(false), || program.apply(pass));
    let mut js_buf = Vec::new();
    let mut mappings = Vec::new();
    let writer = if options.source_map.is_some() {
      JsWriter::new(self.source_map.clone(), "\n", &mut js_buf, Some(&mut mappings))
    } else {
      JsWriter::new(self.source_map.clone(), "\n", &mut js_buf, None)
    };
    let mut emitter = Emitter {
      cfg: Config::default().with_target(options.target).with_minify(false),
      comments: Some(&self.comments),
      cm: self.source_map.clone(),
      wr: writer,
    };
    if let Err(error) = program.emit_with(&mut emitter) {
      return Err(EmitError {
        message: format!("failed to emit code: {}", error),
      });
    }

    let js = String::from_utf8(js_buf).expect("invalid utf8 character detected");
    if let Some(sm) = &options.source_map {
      let mut source_map_json = Vec::new();
      if let Err(error) = self
        .source_map
        .build_source_map_with_config(&mut mappings, None, SourceMapGenOptions {})
        .to_writer(&mut source_map_json)
      {
        return Err(EmitError {
          message: format!("failed to build source map: {}", error),
        });
      }
      if sm.eq("inline") {
        let mut src = js;
        src.push_str("\n//# sourceMappingURL=data:application/json;charset=utf-8;base64,");
        src.push_str(&general_purpose::STANDARD.encode(source_map_json));
        Ok((src, None))
      } else {
        let source_map_json_string = match String::from_utf8(source_map_json) {
          Ok(str) => str,
          Err(error) => {
            return Err(EmitError {
              message: format!("failed to convert source map to string: {}", error),
            });
          }
        };
        Ok((js, Some(source_map_json_string)))
      }
    } else {
      Ok((js, None))
    }
  }
}

fn get_es_syntax(jsx: bool) -> EsSyntax {
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

fn get_ts_syntax(tsx: bool) -> TsSyntax {
  TsSyntax {
    decorators: true,
    tsx,
    ..TsSyntax::default()
  }
}

fn get_syntax(filename: &str, lang: Option<String>) -> Syntax {
  let lang = lang.unwrap_or(
    filename
      .split(|c| c == '?' || c == '#')
      .next()
      .unwrap()
      .split('.')
      .last()
      .unwrap_or("js")
      .to_lowercase(),
  );
  match lang.as_str() {
    "js" | "mjs" => Syntax::Es(get_es_syntax(false)),
    "jsx" => Syntax::Es(get_es_syntax(true)),
    "ts" | "mts" => Syntax::Typescript(get_ts_syntax(false)),
    "tsx" => Syntax::Typescript(get_ts_syntax(true)),
    _ => Syntax::Es(get_es_syntax(false)),
  }
}
