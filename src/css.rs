/*
 * lightningcss - An extremely fast CSS parser, transformer, bundler, and minifier written in Rust.
 * @link https://github.com/parcel-bundler/lightningcss
 * @license MPL-2.0
 */

use lightningcss::css_modules::{Config, CssModuleExports, CssModuleReferences};
use lightningcss::dependencies::{Dependency, DependencyOptions};
use lightningcss::error::{Error, ErrorLocation, MinifyErrorKind, ParserError, PrinterErrorKind};
use lightningcss::stylesheet::{MinifyOptions, ParserFlags, ParserOptions, PrinterOptions, PseudoClasses, StyleSheet};
use lightningcss::targets::{Browsers, Features, Targets};
use parcel_sourcemap::SourceMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use wasm_bindgen::JsError;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceMapJson<'a> {
  version: u8,
  mappings: String,
  sources: &'a Vec<String>,
  sources_content: &'a Vec<String>,
  names: &'a Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformResult {
  pub code: String,
  pub map: Option<String>,
  pub exports: Option<CssModuleExports>,
  pub references: Option<CssModuleReferences>,
  pub dependencies: Option<Vec<Dependency>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AnalyzeDependenciesOption {
  Bool(bool),
  Config(AnalyzeDependenciesConfig),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyzeDependenciesConfig {
  preserve_imports: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Drafts {
  #[serde(default)]
  pub custom_media: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NonStandard {
  #[serde(default)]
  deep_selector_combinator: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformOptions {
  pub targets: Option<Browsers>,
  #[serde(default)]
  pub include: u32,
  #[serde(default)]
  pub exclude: u32,
  pub drafts: Option<Drafts>,
  pub non_standard: Option<NonStandard>,
  pub minify: Option<bool>,
  pub source_map: Option<bool>,
  pub css_modules: Option<CssModulesOption>,
  pub analyze_dependencies: Option<AnalyzeDependenciesOption>,
  pub pseudo_classes: Option<OwnedPseudoClasses>,
  pub unused_symbols: Option<HashSet<String>>,
  pub error_recovery: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CssModulesOption {
  Bool(bool),
  Config(CssModulesConfig),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CssModulesConfig {
  pattern: Option<String>,
  dashed_idents: Option<bool>,
  animation: Option<bool>,
  grid: Option<bool>,
  custom_idents: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnedPseudoClasses {
  pub hover: Option<String>,
  pub active: Option<String>,
  pub focus: Option<String>,
  pub focus_visible: Option<String>,
  pub focus_within: Option<String>,
}

impl<'a> Into<PseudoClasses<'a>> for &'a OwnedPseudoClasses {
  fn into(self) -> PseudoClasses<'a> {
    PseudoClasses {
      hover: self.hover.as_deref(),
      active: self.active.as_deref(),
      focus: self.focus.as_deref(),
      focus_visible: self.focus_visible.as_deref(),
      focus_within: self.focus_within.as_deref(),
    }
  }
}

#[derive(Serialize)]
pub struct Warning<'i> {
  message: String,
  #[serde(flatten)]
  data: ParserError<'i>,
  loc: Option<ErrorLocation>,
}

impl<'i> From<Error<ParserError<'i>>> for Warning<'i> {
  fn from(mut e: Error<ParserError<'i>>) -> Self {
    // Convert to 1-based line numbers.
    if let Some(loc) = &mut e.loc {
      loc.line += 1;
    }
    Warning {
      message: e.kind.to_string(),
      data: e.kind,
      loc: e.loc,
    }
  }
}

pub fn compile<'i>(filename: String, code: &'i str, options: &TransformOptions) -> Result<TransformResult, CompileError<'i>> {
  let drafts = options.drafts.as_ref();
  let non_standard = options.non_standard.as_ref();

  let mut flags = ParserFlags::empty();
  flags.set(ParserFlags::CUSTOM_MEDIA, matches!(drafts, Some(d) if d.custom_media));
  flags.set(
    ParserFlags::DEEP_SELECTOR_COMBINATOR,
    matches!(non_standard, Some(v) if v.deep_selector_combinator),
  );

  let mut stylesheet = StyleSheet::parse(
    &code,
    ParserOptions {
      filename: filename.clone(),
      css_modules: if let Some(css_modules) = &options.css_modules {
        match css_modules {
          CssModulesOption::Bool(true) => Some(Config::default()),
          CssModulesOption::Bool(false) => None,
          CssModulesOption::Config(c) => Some(Config {
            pattern: c.pattern.as_ref().map_or(Default::default(), |pattern| {
              lightningcss::css_modules::Pattern::parse(pattern).unwrap()
            }),
            dashed_idents: c.dashed_idents.unwrap_or(false),
            animation: c.animation.unwrap_or(true),
            grid: c.grid.unwrap_or(true),
            custom_idents: c.custom_idents.unwrap_or(true),
          }),
        }
      } else if filename.ends_with(".module.css") {
        Some(Config::default())
      } else {
        None
      },
      source_index: 0,
      error_recovery: options.error_recovery.unwrap_or_default(),
      warnings: None,
      flags,
    },
  )?;

  let targets = Targets {
    browsers: options.targets,
    include: Features::from_bits_truncate(options.include),
    exclude: Features::from_bits_truncate(options.exclude),
  };

  stylesheet.minify(MinifyOptions {
    targets,
    unused_symbols: options.unused_symbols.clone().unwrap_or_default(),
  })?;

  let mut source_map = if options.source_map.unwrap_or(false) {
    let mut sm = SourceMap::new("/");
    sm.add_source(&filename);
    sm.set_source_content(0, code)?;
    Some(sm)
  } else {
    None
  };

  let res = stylesheet.to_css(PrinterOptions {
    minify: options.minify.unwrap_or(false),
    source_map: source_map.as_mut(),
    project_root: None,
    targets: Targets::from(options.targets.clone().unwrap_or_default()),
    analyze_dependencies: if let Some(d) = &options.analyze_dependencies {
      match d {
        AnalyzeDependenciesOption::Bool(b) if *b => Some(DependencyOptions { remove_imports: true }),
        AnalyzeDependenciesOption::Config(c) => Some(DependencyOptions {
          remove_imports: !c.preserve_imports,
        }),
        _ => None,
      }
    } else {
      None
    },
    pseudo_classes: options.pseudo_classes.as_ref().map(|p| p.into()),
  })?;

  let map = if let Some(mut source_map) = source_map {
    Some(source_map_to_json(&mut source_map)?)
  } else {
    None
  };

  Ok(TransformResult {
    code: res.code,
    map,
    exports: res.exports,
    references: res.references,
    dependencies: res.dependencies,
  })
}

#[inline]
fn source_map_to_json<'i>(source_map: &mut SourceMap) -> Result<String, CompileError<'i>> {
  let mut vlq_output: Vec<u8> = Vec::new();
  source_map.write_vlq(&mut vlq_output)?;

  let sm = SourceMapJson {
    version: 3,
    mappings: unsafe { String::from_utf8_unchecked(vlq_output) },
    sources: source_map.get_sources(),
    sources_content: source_map.get_sources_content(),
    names: source_map.get_names(),
  };

  Ok(serde_json::to_string(&sm).unwrap())
}

#[derive(Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttrConfig {
  pub code: String,
  pub targets: Option<Browsers>,
  pub minify: Option<bool>,
  pub analyze_dependencies: Option<bool>,
}

#[derive(Debug)]
pub enum CompileError<'i> {
  ParseError(Error<ParserError<'i>>),
  MinifyError(Error<MinifyErrorKind>),
  PrinterError(Error<PrinterErrorKind>),
  SourceMapError(parcel_sourcemap::SourceMapError),
}

impl<'i> CompileError<'i> {
  fn reason(&self) -> String {
    match self {
      CompileError::ParseError(e) => format!("{}", e),
      CompileError::MinifyError(e) => format!("{}", e),
      CompileError::PrinterError(e) => format!("{}", e),
      _ => "Unknown error".into(),
    }
  }
}

impl<'i> From<Error<ParserError<'i>>> for CompileError<'i> {
  fn from(e: Error<ParserError<'i>>) -> CompileError<'i> {
    CompileError::ParseError(e)
  }
}

impl<'i> From<Error<MinifyErrorKind>> for CompileError<'i> {
  fn from(err: Error<MinifyErrorKind>) -> CompileError<'i> {
    CompileError::MinifyError(err)
  }
}

impl<'i> From<Error<PrinterErrorKind>> for CompileError<'i> {
  fn from(err: Error<PrinterErrorKind>) -> CompileError<'i> {
    CompileError::PrinterError(err)
  }
}

impl<'i> From<parcel_sourcemap::SourceMapError> for CompileError<'i> {
  fn from(e: parcel_sourcemap::SourceMapError) -> CompileError<'i> {
    CompileError::SourceMapError(e)
  }
}

impl<'i> From<CompileError<'i>> for JsError {
  fn from(e: CompileError) -> JsError {
    match e {
      CompileError::SourceMapError(e) => JsError::new(&e.to_string()),
      _ => JsError::new(&e.reason()),
    }
  }
}
