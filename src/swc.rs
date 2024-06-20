use crate::error::{DiagnosticBuffer, ErrorBuffer};
use crate::hmr::{HmrOptions, HMR};
use crate::minifier::{Minifier, MinifierOptions};
use crate::resolver::Resolver;
use crate::resolver_fold::ResolverFolder;

use base64::{engine::general_purpose, Engine as _};
use std::{cell::RefCell, path::Path, rc::Rc};
use swc_common::comments::SingleThreadedComments;
use swc_common::errors::{Handler, HandlerFlags};
use swc_common::{chain, FileName, Globals, Mark, SourceMap};
use swc_ecma_transforms::optimization::simplify::dce;
use swc_ecma_transforms::pass::Optional;
use swc_ecma_transforms::proposals::decorators;
use swc_ecma_transforms::typescript::{tsx, typescript};
use swc_ecma_transforms::{compat, fixer, helpers, hygiene, react, Assumptions};
use swc_ecmascript::ast::{EsVersion, Module, Program};
use swc_ecmascript::codegen;
use swc_ecmascript::codegen::text_writer::JsWriter;
use swc_ecmascript::codegen::Node;
use swc_ecmascript::parser::lexer::Lexer;
use swc_ecmascript::parser::{EsConfig, StringInput, Syntax, TsConfig};
use swc_ecmascript::visit::{as_folder, Fold, FoldWith};

/// Options for transpiling a module.
#[derive(Clone)]
pub struct EmitOptions {
  pub target: EsVersion,
  pub jsx_pragma: Option<String>,
  pub jsx_pragma_frag: Option<String>,
  pub jsx_import_source: Option<String>,
  pub minify: Option<MinifierOptions>,
  pub tree_shaking: Option<bool>,
  pub is_dev: Option<bool>,
  pub hmr: Option<HmrOptions>,
  pub source_map: Option<String>,
}

impl Default for EmitOptions {
  fn default() -> Self {
    EmitOptions {
      target: EsVersion::Es2022,
      jsx_pragma: None,
      jsx_pragma_frag: None,
      jsx_import_source: None,
      minify: None,
      tree_shaking: None,
      is_dev: None,
      hmr: None,
      source_map: None,
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
  pub fn parse(specifier: &str, source: &str, lang: Option<String>) -> Result<Self, anyhow::Error> {
    let source_map = SourceMap::default();
    let source_file = source_map.new_source_file(FileName::Real(Path::new(specifier).to_path_buf()), source.into());
    let sm = &source_map;
    let error_buffer = ErrorBuffer::new(specifier);
    let syntax = get_syntax(specifier, lang);
    let input = StringInput::from(&*source_file);
    let comments = SingleThreadedComments::default();
    let lexer = Lexer::new(syntax, EsVersion::EsNext, input, Some(&comments));
    let mut parser = swc_ecmascript::parser::Parser::new_from(lexer);
    let handler = Handler::with_emitter_and_flags(
      Box::new(error_buffer.clone()),
      HandlerFlags {
        can_emit_warnings: true,
        dont_buffer_diagnostics: true,
        ..HandlerFlags::default()
      },
    );
    let module = parser
      .parse_module()
      .map_err(move |err| {
        let mut diagnostic = err.into_diagnostic(&handler);
        diagnostic.emit();
        DiagnosticBuffer::from_error_buffer(error_buffer, |span| sm.lookup_char_pos(span.lo))
      })
      .unwrap();

    Ok(SWC {
      specifier: specifier.into(),
      module,
      source_map: Rc::new(source_map),
      comments,
    })
  }

  /// Transpile a JS/TS module.
  pub fn transform(
    self,
    resolver: Rc<RefCell<Resolver>>,
    options: &EmitOptions,
  ) -> Result<(String, Option<String>), anyhow::Error> {
    swc_common::GLOBALS.set(&Globals::new(), || {
      let unresolved_mark = Mark::new();
      let top_level_mark = Mark::fresh(Mark::root());
      let specifier_is_remote = resolver.borrow().specifier_is_remote;
      let is_dev = options.is_dev.unwrap_or_default();
      let is_ts =
        self.specifier.ends_with(".ts") || self.specifier.ends_with(".mts") || self.specifier.ends_with(".tsx");
      let is_jsx = self.specifier.ends_with(".tsx") || self.specifier.ends_with(".jsx");
      let react_options = if let Some(jsx_import_source) = &options.jsx_import_source {
        let mut resolver = resolver.borrow_mut();
        let runtime = if is_dev { "/jsx-dev-runtime" } else { "/jsx-runtime" };
        let import_source = resolver.resolve(&(jsx_import_source.to_owned() + runtime), false, None);
        let import_source = import_source
          .split("?")
          .next()
          .unwrap_or(&import_source)
          .strip_suffix(runtime)
          .unwrap_or(&import_source)
          .to_string();
        if !is_jsx {
          resolver.deps.pop();
        }
        react::Options {
          runtime: Some(react::Runtime::Automatic),
          import_source: Some(import_source),
          ..Default::default()
        }
      } else {
        react::Options {
          pragma: options.jsx_pragma.clone(),
          pragma_frag: options.jsx_pragma_frag.clone(),
          ..Default::default()
        }
      };
      let assumptions = Assumptions::all();
      let passes = chain!(
        swc_ecma_transforms::resolver(unresolved_mark, top_level_mark, is_ts),
        Optional::new(react::jsx_src(is_dev, self.source_map.clone()), is_jsx),
        ResolverFolder {
          resolver: resolver.clone(),
          mark_src_location: None,
        },
        decorators::decorators(decorators::Config {
          legacy: true,
          emit_metadata: false,
          use_define_for_class_fields: false,
        }),
        Optional::new(compat::es2016(), options.target < EsVersion::Es2016),
        Optional::new(
          compat::es2017(
            compat::es2017::Config {
              async_to_generator: compat::es2017::async_to_generator::Config {
                ignore_function_name: assumptions.ignore_function_name,
                ignore_function_length: assumptions.ignore_function_length
              }
            },
            Some(&self.comments),
            unresolved_mark,
          ),
          options.target < EsVersion::Es2017
        ),
        Optional::new(
          compat::es2018(compat::es2018::Config {
            object_rest_spread: compat::es2018::object_rest_spread::Config {
              no_symbol: assumptions.object_rest_no_symbols,
              set_property: assumptions.set_spread_properties,
              pure_getters: assumptions.pure_getters,
            }
          }),
          options.target < EsVersion::Es2018
        ),
        Optional::new(compat::es2019::es2019(), options.target < EsVersion::Es2019),
        Optional::new(
          compat::es2020::es2020(
            compat::es2020::Config {
              nullish_coalescing: compat::es2020::nullish_coalescing::Config {
                no_document_all: assumptions.no_document_all
              },
              optional_chaining: compat::es2020::optional_chaining::Config {
                no_document_all: assumptions.no_document_all,
                pure_getter: assumptions.pure_getters
              }
            },
            unresolved_mark
          ),
          options.target < EsVersion::Es2020
        ),
        Optional::new(compat::es2021::es2021(), options.target < EsVersion::Es2021),
        Optional::new(
          compat::es2022::es2022(
            Some(&self.comments),
            compat::es2022::Config {
              class_properties: compat::es2022::class_properties::Config {
                private_as_properties: assumptions.private_fields_as_properties,
                constant_super: assumptions.constant_super,
                set_public_fields: assumptions.set_public_class_fields,
                no_document_all: assumptions.no_document_all,
                pure_getter: assumptions.pure_getters,
                static_blocks_mark: Mark::new(),
              }
            },
            unresolved_mark
          ),
          options.target < EsVersion::Es2022
        ),
        compat::reserved_words::reserved_words(),
        helpers::inject_helpers(top_level_mark),
        Optional::new(typescript::strip(top_level_mark), is_ts && !is_jsx),
        Optional::new(
          tsx(
            self.source_map.clone(),
            typescript::Config::default(),
            typescript::TsxConfig {
              pragma: react_options.pragma.clone(),
              pragma_frag: react_options.pragma_frag.clone(),
            },
            Some(&self.comments),
            top_level_mark
          ),
          is_ts && is_jsx
        ),
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
          options
            .hmr
            .as_ref()
            .unwrap_or(&HmrOptions::default())
            .react_refresh
            .unwrap_or_default()
            && !specifier_is_remote
        ),
        Optional::new(
          react::jsx(
            self.source_map.clone(),
            Some(&self.comments),
            react::Options {
              development: Some(is_dev),
              ..react_options
            },
            top_level_mark,
            unresolved_mark,
          ),
          is_jsx
        ),
        Optional::new(
          HMR {
            specifier: self.specifier.clone(),
            options: options.hmr.as_ref().unwrap_or(&HmrOptions::default()).clone(),
          },
          options.hmr.is_some() && !specifier_is_remote
        ),
        Optional::new(
          dce::dce(Default::default(), unresolved_mark),
          options.tree_shaking.unwrap_or_default()
        ),
        Optional::new(
          as_folder(Minifier {
            cm: self.source_map.clone(),
            comments: Some(self.comments.clone()),
            unresolved_mark,
            top_level_mark,
            options: options.minify.unwrap_or_default(),
          }),
          options.minify.is_some()
        ),
        hygiene::hygiene_with_config(hygiene::Config {
          keep_class_names: true,
          top_level_mark,
          ..Default::default()
        }),
        fixer(Some(&self.comments)),
      );

      // emit code
      let (mut code, map) = self.emit(passes, options).unwrap();

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
        code = code.replace(
          format!("\"{}\"", jsx_runtime).as_str(),
          format!("\"{}\"", import_url).as_str(),
        );
      }

      Ok((code, map))
    })
  }

  /// Emit code with a given set of passes.
  fn emit<T: Fold>(&self, mut passes: T, options: &EmitOptions) -> Result<(String, Option<String>), anyhow::Error> {
    let eol = "\n";
    let program = Program::Module(self.module.clone());
    let program = helpers::HELPERS.set(&helpers::Helpers::new(false), || program.fold_with(&mut passes));
    let mut js_buf = Vec::new();
    let mut map_buf = Vec::new();
    let writer = if options.source_map.is_some() {
      JsWriter::new(self.source_map.clone(), eol, &mut js_buf, Some(&mut map_buf))
    } else {
      JsWriter::new(self.source_map.clone(), eol, &mut js_buf, None)
    };
    let mut emitter = codegen::Emitter {
      cfg: codegen::Config::default()
        .with_target(options.target)
        .with_minify(options.minify.is_some()),
      comments: Some(&self.comments),
      cm: self.source_map.clone(),
      wr: writer,
    };
    program.emit_with(&mut emitter).unwrap();

    let js = String::from_utf8(js_buf).unwrap();
    if let Some(sm) = &options.source_map {
      let mut source_map = Vec::new();
      self
        .source_map
        .build_source_map_from(&mut map_buf, None)
        .to_writer(&mut source_map)
        .unwrap();
      if sm.eq("inline") {
        let mut src = js;
        src.push_str("\n//# sourceMappingURL=data:application/json;base64,");
        src.push_str(&general_purpose::STANDARD.encode(source_map));
        Ok((src, None))
      } else {
        Ok((js, Some(String::from_utf8(source_map).unwrap())))
      }
    } else {
      Ok((js, None))
    }
  }
}

fn get_es_config(jsx: bool) -> EsConfig {
  EsConfig {
    fn_bind: true,
    export_default_from: true,
    allow_super_outside_method: true,
    allow_return_outside_function: true,
    decorators: true,
    jsx,
    ..EsConfig::default()
  }
}

fn get_ts_config(tsx: bool) -> TsConfig {
  TsConfig {
    decorators: true,
    tsx,
    ..TsConfig::default()
  }
}

fn get_syntax(specifier: &str, lang: Option<String>) -> Syntax {
  let lang = if let Some(lang) = lang {
    lang
  } else {
    specifier
      .split(|c| c == '?' || c == '#')
      .next()
      .unwrap()
      .split('.')
      .last()
      .unwrap_or("js")
      .to_lowercase()
  };
  match lang.as_str() {
    "js" | "mjs" => Syntax::Es(get_es_config(false)),
    "jsx" => Syntax::Es(get_es_config(true)),
    "ts" | "mts" => Syntax::Typescript(get_ts_config(false)),
    "tsx" => Syntax::Typescript(get_ts_config(true)),
    _ => Syntax::Es(get_es_config(false)),
  }
}
