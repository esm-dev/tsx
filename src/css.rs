/*
 * lightningcss - An extremely fast CSS parser, transformer, bundler, and minifier written in Rust.
 * @link https://github.com/parcel-bundler/lightningcss
 * @license MPL-2.0
 */

use lightningcss::css_modules::CssModuleExports;
use lightningcss::dependencies::Dependency;
use lightningcss::error::{Error, MinifyErrorKind, ParserError, PrinterErrorKind};
use lightningcss::stylesheet::{MinifyOptions, ParserFlags, ParserOptions, PrinterOptions, PseudoClasses, StyleSheet};
use lightningcss::targets::Browsers;
use lightningcss::targets::Targets;
use parcel_sourcemap::SourceMap;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

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
  pub dependencies: Option<Vec<Dependency>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyOptions {
  /// Whether to remove `@import` rules.
  pub remove_imports: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
  pub targets: Option<Browsers>,
  pub minify: Option<bool>,
  pub source_map: Option<bool>,
  pub css_modules: Option<CssModulesOption>,
  pub analyze_dependencies: Option<DependencyOptions>,
  pub pseudo_classes: Option<OwnedPseudoClasses>,
  pub unused_symbols: Option<HashSet<String>>,
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
  #[serde(default)]
  dashed_idents: bool,
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

pub fn compile<'i>(filename: String, code: &'i str, config: &Config) -> Result<TransformResult, CompileError<'i>> {
  let warnings = Some(Arc::new(RwLock::new(Vec::new())));
  let mut stylesheet = StyleSheet::parse(
    &code,
    ParserOptions {
      filename: filename.clone(),
      css_modules: if let Some(css_modules) = &config.css_modules {
        match css_modules {
          CssModulesOption::Bool(true) => Some(lightningcss::css_modules::Config::default()),
          CssModulesOption::Bool(false) => None,
          CssModulesOption::Config(c) => Some(lightningcss::css_modules::Config {
            pattern: c.pattern.as_ref().map_or(Default::default(), |pattern| {
              lightningcss::css_modules::Pattern::parse(pattern).unwrap()
            }),
            dashed_idents: c.dashed_idents,
          }),
        }
      } else if filename.ends_with(".module.css") {
        Some(lightningcss::css_modules::Config::default())
      } else {
        None
      },
      source_index: 0,
      error_recovery: false,
      warnings: warnings.clone(),
      flags: ParserFlags::NESTING | ParserFlags::CUSTOM_MEDIA,
    },
  )?;
  stylesheet.minify(MinifyOptions {
    targets: Targets::from(config.targets.clone().unwrap_or_default()),
    unused_symbols: config.unused_symbols.clone().unwrap_or_default(),
  })?;

  let mut source_map = if config.source_map.unwrap_or(false) {
    let mut sm = SourceMap::new("/");
    sm.add_source(&filename);
    sm.set_source_content(0, code)?;
    Some(sm)
  } else {
    None
  };

  let res = stylesheet.to_css(PrinterOptions {
    minify: config.minify.unwrap_or(false),
    source_map: source_map.as_mut(),
    project_root: None,
    targets: Targets::from(config.targets.clone().unwrap_or_default()),
    analyze_dependencies: if let Some(analyze_dependencies) = &config.analyze_dependencies {
      Some(lightningcss::dependencies::DependencyOptions {
        remove_imports: analyze_dependencies.remove_imports,
      })
    } else {
      None
    },
    pseudo_classes: config.pseudo_classes.as_ref().map(|p| p.into()),
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AttrResult {
  code: String,
  dependencies: Option<Vec<Dependency>>,
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

impl<'i> From<CompileError<'i>> for wasm_bindgen::JsValue {
  fn from(e: CompileError) -> wasm_bindgen::JsValue {
    match e {
      CompileError::SourceMapError(e) => js_sys::Error::new(&e.to_string()).into(),
      _ => js_sys::Error::new(&e.reason()).into(),
    }
  }
}
