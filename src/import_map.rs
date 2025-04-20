// Copied from: https://github.com/denoland/import_map
// Add `ext` feature for esm.sh imports
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use crate::specifier;
use indexmap::IndexMap;
use serde_json::{Map, Value};
use std::cmp::Ordering;
use std::collections::HashSet;
use thiserror::Error;
use url::Url;

#[derive(Clone, PartialEq, Eq)]
pub enum ImportMapDiagnostic {
  EmptySpecifier,
  InvalidScope(String, String),
  InvalidTargetAddress(String, String),
  InvalidAddress(String, String),
  InvalidAddressNotString(String, String),
  InvalidTopLevelKey(String),
}

#[derive(Error, Debug)]
pub enum ImportMapError {
  #[error(
    "Relative import path \"{}\" not prefixed with / or ./ or ../ and not in import map{}",
    .0,
    .1.as_ref().map(|referrer| format!(" from \"{}\"", referrer)).unwrap_or_default(),
  )]
  UnmappedBareSpecifier(String, Option<String>),
  #[error("{0}")]
  Other(String),
}

type SpecifierMap = IndexMap<String, Option<Url>>;
type ScopesMap = IndexMap<String, SpecifierMap>;
type UnresolvedSpecifierMap = IndexMap<String, Option<String>>;
type UnresolvedScopesMap = IndexMap<String, UnresolvedSpecifierMap>;

#[derive(Debug, Clone)]
pub struct ImportMap {
  base_url: Url,
  imports: SpecifierMap,
  scopes: ScopesMap,
}

impl ImportMap {
  pub fn base_url(&self) -> &Url {
    &self.base_url
  }

  pub fn resolve(&self, specifier: &str, referrer: &Url) -> Result<Url, ImportMapError> {
    let as_url: Option<Url> = try_url_like_specifier(specifier, referrer);
    let normalized_specifier = if let Some(url) = as_url.as_ref() {
      url.to_string()
    } else {
      specifier.to_string()
    };

    let scopes_match = resolve_scopes_match(&self.scopes, &normalized_specifier, referrer.as_ref())?;

    // match found in scopes map
    if let Some(scopes_match) = scopes_match {
      return Ok(scopes_match);
    }

    let imports_match = resolve_imports_match(&self.imports, &normalized_specifier)?;

    // match found in import map
    if let Some(imports_match) = imports_match {
      return Ok(imports_match);
    }

    // The specifier was able to be turned into a URL, but wasn't remapped into anything.
    if let Some(as_url) = as_url {
      return Ok(as_url);
    }

    Err(ImportMapError::UnmappedBareSpecifier(
      specifier.to_string(),
      Some(referrer.to_string()),
    ))
  }
}

pub fn parse_from_value(base_url: Url, json_value: Value) -> Result<ImportMap, ImportMapError> {
  let mut diagnostics = vec![];
  let (unresolved_imports, unresolved_scopes) = parse_value(json_value, &mut diagnostics)?;
  let imports = parse_specifier_map(unresolved_imports, &base_url, &mut diagnostics);
  let scopes = parse_scope_map(unresolved_scopes, &base_url, &mut diagnostics)?;

  Ok(ImportMap { base_url, imports, scopes })
}

fn parse_value(
  mut v: Value,
  diagnostics: &mut Vec<ImportMapDiagnostic>,
) -> Result<(UnresolvedSpecifierMap, UnresolvedScopesMap), ImportMapError> {
  match v {
    Value::Object(_) => {}
    _ => {
      return Err(ImportMapError::Other("Import map JSON must be an object".to_string()));
    }
  }

  let imports = if v.get("imports").is_some() {
    match v["imports"].take() {
      Value::Object(imports_map) => parse_specifier_map_json(imports_map, diagnostics),
      _ => {
        return Err(ImportMapError::Other("Import map's 'imports' must be an object".to_string()));
      }
    }
  } else {
    IndexMap::new()
  };

  let scopes = if v.get("scopes").is_some() {
    match v["scopes"].take() {
      Value::Object(scopes_map) => parse_scopes_map_json(scopes_map, diagnostics)?,
      _ => {
        return Err(ImportMapError::Other("Import map's 'scopes' must be an object".to_string()));
      }
    }
  } else {
    IndexMap::new()
  };

  let mut keys: HashSet<String> = v.as_object().unwrap().keys().map(|k| k.to_string()).collect();
  keys.remove("imports");
  keys.remove("scopes");
  for key in keys {
    diagnostics.push(ImportMapDiagnostic::InvalidTopLevelKey(key));
  }

  Ok((imports, scopes))
}

/// Convert provided JSON map to key values
fn parse_specifier_map_json(json_map: Map<String, Value>, diagnostics: &mut Vec<ImportMapDiagnostic>) -> UnresolvedSpecifierMap {
  let mut map: IndexMap<String, Option<String>> = IndexMap::new();

  // Order is preserved because of "preserve_order" feature of "serde_json".
  for (specifier_key, value) in json_map.into_iter() {
    map.insert(
      specifier_key.clone(),
      match value {
        Value::String(address) => Some(address),
        _ => {
          diagnostics.push(ImportMapDiagnostic::InvalidAddressNotString(value.to_string(), specifier_key));
          None
        }
      },
    );
  }

  map
}

/// Convert provided JSON map to key value strings.
fn parse_scopes_map_json(
  scopes_map: Map<String, Value>,
  diagnostics: &mut Vec<ImportMapDiagnostic>,
) -> Result<UnresolvedScopesMap, ImportMapError> {
  let mut map = UnresolvedScopesMap::new();

  // Order is preserved because of "preserve_order" feature of "serde_json".
  for (scope_prefix, mut potential_specifier_map) in scopes_map.into_iter() {
    let potential_specifier_map = match potential_specifier_map.take() {
      Value::Object(obj) => obj,
      _ => {
        return Err(ImportMapError::Other(format!(
          "The value for the {:?} scope prefix must be an object",
          scope_prefix
        )));
      }
    };

    let specifier_map = parse_specifier_map_json(potential_specifier_map, diagnostics);

    map.insert(scope_prefix.to_string(), specifier_map);
  }

  Ok(map)
}

/// Convert provided key value string imports to valid SpecifierMap.
///
/// From specification:
/// - order of iteration must be retained
/// - SpecifierMap's keys are sorted in longest and alphabetic order
fn parse_specifier_map(imports: UnresolvedSpecifierMap, base_url: &Url, diagnostics: &mut Vec<ImportMapDiagnostic>) -> SpecifierMap {
  let mut normalized_map: SpecifierMap = SpecifierMap::new();

  for (_, (key, value)) in imports.into_iter().enumerate() {
    let normalized_key = match normalize_specifier_key(&key, base_url) {
      Ok(s) => s,
      Err(err) => {
        diagnostics.push(err);
        continue;
      }
    };
    let potential_address = match &value {
      Some(address) => address,
      None => {
        normalized_map.insert(normalized_key, None);
        continue;
      }
    };

    let address_url = match try_url_like_specifier(potential_address, base_url) {
      Some(url) => url,
      None => {
        diagnostics.push(ImportMapDiagnostic::InvalidAddress(potential_address.to_string(), key.to_string()));
        normalized_map.insert(normalized_key, None);
        continue;
      }
    };

    let address_url_string = address_url.to_string();
    if key.ends_with('/') && !address_url_string.ends_with('/') {
      diagnostics.push(ImportMapDiagnostic::InvalidTargetAddress(address_url_string, key.to_string()));
      normalized_map.insert(normalized_key, None);
      continue;
    }

    normalized_map.insert(normalized_key, Some(address_url));
  }

  // Sort in longest and alphabetical order.
  normalized_map.sort_by(|k1, _v1, k2, _v2| match k1.cmp(k2) {
    Ordering::Greater => Ordering::Less,
    Ordering::Less => Ordering::Greater,
    // JSON guarantees that there can't be duplicate keys
    Ordering::Equal => unreachable!(),
  });

  normalized_map
}

/// Convert provided key value string scopes to valid ScopeMap.
///
/// From specification:
/// - order of iteration must be retained
/// - ScopeMap's keys are sorted in longest and alphabetic order
fn parse_scope_map(
  scope_map: UnresolvedScopesMap,
  base_url: &Url,
  diagnostics: &mut Vec<ImportMapDiagnostic>,
) -> Result<ScopesMap, ImportMapError> {
  let mut normalized_map: ScopesMap = ScopesMap::new();

  // Order is preserved because of "preserve_order" feature of "serde_json".
  for (_, (raw_scope_prefix, potential_specifier_map)) in scope_map.into_iter().enumerate() {
    let scope_prefix_url = match base_url.join(&raw_scope_prefix) {
      Ok(url) => url.to_string(),
      _ => {
        diagnostics.push(ImportMapDiagnostic::InvalidScope(raw_scope_prefix, base_url.to_string()));
        continue;
      }
    };

    let norm_map = parse_specifier_map(potential_specifier_map, base_url, diagnostics);

    normalized_map.insert(scope_prefix_url, norm_map);
  }

  // Sort in longest and alphabetical order.
  normalized_map.sort_by(|k1, _v1, k2, _v2| match k1.cmp(k2) {
    Ordering::Greater => Ordering::Less,
    Ordering::Less => Ordering::Greater,
    // JSON guarantees that there can't be duplicate keys
    Ordering::Equal => unreachable!(),
  });

  Ok(normalized_map)
}

fn try_url_like_specifier(specifier: &str, base: &Url) -> Option<Url> {
  if specifier.starts_with('/') || specifier.starts_with("./") || specifier.starts_with("../") {
    if let Ok(url) = base.join(specifier) {
      return Some(url);
    }
  }

  if let Ok(url) = Url::parse(specifier) {
    return Some(url);
  }

  None
}

/// Parse provided key as import map specifier.
///
/// Specifiers must be valid URLs (eg. "`https://deno.land/x/std/testing/asserts.ts`")
/// or "bare" specifiers (eg. "moment").
fn normalize_specifier_key(specifier_key: &str, base_url: &Url) -> Result<String, ImportMapDiagnostic> {
  // ignore empty keys
  if specifier_key.is_empty() {
    Err(ImportMapDiagnostic::EmptySpecifier)
  } else if let Some(url) = try_url_like_specifier(specifier_key, base_url) {
    Ok(url.to_string())
  } else {
    // "bare" specifier
    Ok(specifier_key.to_string())
  }
}

fn append_specifier_to_base(base: &Url, specifier: &str) -> Result<Url, url::ParseError> {
  // Percent-decode first. Specifier might be pre-encoded and could get encoded
  // again.
  let mut base = base.clone();
  let specifier = percent_encoding::percent_decode_str(specifier).decode_utf8_lossy();
  let is_relative_or_absolute_specifier = specifier.starts_with("../") || specifier.starts_with("./") || specifier.starts_with('/');

  // The specifier could be a windows path such as "C:/a/test.ts" in which
  // case we don't want to use `join` because it will make the specifier
  // the url since it contains what looks to be a uri scheme. To work around
  // this, we append the specifier to the path segments of the base url when
  // the specifier is not relative or absolute.
  let mut maybe_query_string_and_fragment = None;
  if !is_relative_or_absolute_specifier && base.path_segments_mut().is_ok() {
    {
      let mut segments = base.path_segments_mut().unwrap();
      segments.pop_if_empty();

      // Handle query-string and fragment first, otherwise they would be percent-encoded
      // by `extend()`
      let prefix = match specifier.find(&['?', '#'][..]) {
        Some(idx) => {
          maybe_query_string_and_fragment = Some(&specifier[idx..]);
          &specifier[..idx]
        }
        None => &specifier,
      };
      segments.extend(prefix.split('/'));
    }

    if let Some(query_string_and_fragment) = maybe_query_string_and_fragment {
      Ok(base.join(query_string_and_fragment)?)
    } else {
      Ok(base)
    }
  } else {
    Ok(base.join(&specifier)?)
  }
}

fn resolve_scopes_match(scopes: &ScopesMap, normalized_specifier: &str, referrer: &str) -> Result<Option<Url>, ImportMapError> {
  // exact-match
  if let Some(scope_imports) = scopes.get(referrer) {
    let scope_match = resolve_imports_match(&scope_imports, normalized_specifier)?;
    // Return only if there was actual match (not None).
    if scope_match.is_some() {
      return Ok(scope_match);
    }
  }

  for (normalized_scope_key, scope_imports) in scopes.iter() {
    if normalized_scope_key.ends_with('/') && referrer.starts_with(normalized_scope_key) {
      let scope_match = resolve_imports_match(&scope_imports, normalized_specifier)?;
      // Return only if there was actual match (not None).
      if scope_match.is_some() {
        return Ok(scope_match);
      }
    }
  }

  Ok(None)
}

fn resolve_imports_match(specifier_map: &SpecifierMap, normalized_specifier: &str) -> Result<Option<Url>, ImportMapError> {
  // exact-match
  if let Some(value) = specifier_map.get(normalized_specifier) {
    if let Some(address) = &value {
      return Ok(Some(address.clone()));
    } else {
      return Err(ImportMapError::Other(format!(
        "Blocked by null entry for \"{:?}\"",
        normalized_specifier
      )));
    }
  }

  // Package-prefix match
  // "most-specific wins", i.e. when there are multiple matching keys,
  // choose the longest.
  for (specifier_key, value) in specifier_map.iter() {
    if !specifier_key.ends_with('/') {
      continue;
    }

    if !normalized_specifier.starts_with(specifier_key) {
      continue;
    }

    let resolution_result = value
      .as_ref()
      .ok_or_else(|| ImportMapError::Other(format!("Blocked by null entry for \"{:?}\"", specifier_key)))?;

    // Enforced by parsing.
    assert!(resolution_result.to_string().ends_with('/'));

    let after_prefix = &normalized_specifier[specifier_key.len()..];

    let url = match append_specifier_to_base(resolution_result, after_prefix) {
      Ok(url) => url,
      Err(_) => {
        return Err(ImportMapError::Other(format!(
          "Failed to resolve the specifier \"{:?}\" as its after-prefix
            portion \"{:?}\" could not be URL-parsed relative to the URL prefix
            \"{}\" mapped to by the prefix \"{}\"",
          normalized_specifier, after_prefix, resolution_result, specifier_key
        )));
      }
    };

    if !url.as_str().starts_with(resolution_result.as_str()) {
      return Err(ImportMapError::Other(format!(
        "The specifier \"{:?}\" backtracks above its prefix \"{:?}\"",
        normalized_specifier, specifier_key
      )));
    }

    return Ok(Some(url));
  }

  // expand-match
  // expand specifiers with tailing slash
  // e.g. "react": "https://esm.sh/react",
  //      "react/": "https://esm.sh/react/" (expanded)
  if !specifier::is_http_specifier(normalized_specifier)
    && !specifier::is_relpath_specifier(normalized_specifier)
    && !specifier::is_abspath_specifier(normalized_specifier)
  {
    for (specifier_key, value) in specifier_map.iter() {
      if !specifier_key.ends_with('/') {
        if let Some(address) = value {
          if normalized_specifier.starts_with((specifier_key.to_owned() + "/").as_str())
            && (address.scheme().eq("https") || address.scheme().eq("http"))
          {
            let mut url = address.clone();
            url.set_path(&(url.path().to_owned() + normalized_specifier.strip_prefix(specifier_key).unwrap()));
            return Ok(Some(url));
          }
        }
      }
    }
  }

  Ok(None)
}
