use crate::import_map::ImportMap;
use crate::specifier::{is_abspath_specifier, is_http_specifier, is_relpath_specifier};
use base64::{Engine as _, engine::general_purpose};
use path_slash::PathBufExt;
use pathdiff::diff_paths;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use url::Url;

/// A Resolver to resolve esm import/export URL.
pub struct Resolver {
  /// the text specifier associated with the import/export statement.
  pub specifier: String,
  /// a ordered dependencies of the module
  pub deps: Vec<(String, String)>,
  /// the graph versions
  pub version_map: Option<HashMap<String, String>>,
  /// the import map
  pub import_map: Option<ImportMap>,
}

impl Resolver {
  /// Create a new Resolver.
  pub fn new(specifier: &str, import_map: Option<ImportMap>, version_map: Option<HashMap<String, String>>) -> Self {
    Resolver {
      specifier: specifier.into(),
      deps: Vec::new(),
      import_map,
      version_map,
    }
  }

  /// Resolve module specifier to a URL.
  pub fn resolve(&mut self, specifier: &str) -> String {
    let referrer = if is_http_specifier(&self.specifier) {
      Url::from_str(self.specifier.as_str()).unwrap()
    } else {
      Url::from_str(&("file://".to_owned() + self.specifier.as_str())).unwrap()
    };
    let resolved_url = if let Some(import_map) = &self.import_map {
      if let Ok(ret) = import_map.resolve(specifier, &referrer) {
        ret.to_string()
      } else {
        specifier.into()
      }
    } else {
      specifier.into()
    };
    let mut resolved_url = if resolved_url.starts_with("file://") {
      let pathname = resolved_url.strip_prefix("file://").unwrap();
      if !is_http_specifier(&self.specifier) {
        let mut buf = PathBuf::from(self.specifier.to_owned());
        buf.pop();
        let path = diff_paths(&pathname, buf).unwrap().to_slash().unwrap().to_string();
        let rel_path = if !path.starts_with("./") && !path.starts_with("../") {
          "./".to_owned() + path.as_str()
        } else {
          path
        };
        if rel_path.len() < pathname.len() {
          rel_path
        } else {
          pathname.to_owned()
        }
      } else {
        pathname.to_owned()
      }
    } else {
      resolved_url.to_owned()
    };
    let is_filepath = is_relpath_specifier(&resolved_url) || is_abspath_specifier(&resolved_url);

    let mut extra_query: Option<String> = None;
    let raw_query: Option<Vec<String>> = if let Some(i) = resolved_url.find('?') {
      let query = resolved_url[i + 1..].to_owned();
      resolved_url = resolved_url[..i].to_owned();
      Some(query.split('&').map(|s| s.to_owned()).collect())
    } else {
      None
    };

    if is_filepath
      && !raw_query
        .as_ref()
        .is_some_and(|q| q.iter().any(|p| p == "url" || p == "raw" || p == "rpc"))
    {
      if let Some(ext) = Path::new(&resolved_url).extension() {
        let extname = ext.to_str().unwrap();
        match extname {
          "js" | "mjs" | "ts" | "mts" | "jsx" | "tsx" | "vue" | "svelte" | "css" | "json" | "md" => {
            if extname == "css" {
              extra_query = Some("module".to_owned());
            } else if extname != "json"
              && (extname != "md"
                || raw_query
                  .as_ref()
                  .is_some_and(|q| q.iter().any(|p| p == "jsx" || p == "vue" || p == "svelte")))
            {
              if let Some(base_url) = self.import_map.as_ref().map(|im| im.base_url()) {
                let base_path = base_url.path();
                if !base_path.eq("/anonymous_import_map.json") {
                  let base_path_base64 = general_purpose::URL_SAFE_NO_PAD.encode(base_path.as_bytes());
                  extra_query = Some("im=".to_owned() + base_path_base64.as_str());
                }
              }
            }
            let mut version: Option<&String> = None;
            if let Some(version_map) = &self.version_map {
              let fullpath = referrer.join(&resolved_url).unwrap().path().to_owned();
              if version_map.contains_key(&fullpath) {
                version = version_map.get(&fullpath)
              } else {
                version = version_map.get("*")
              }
            };
            if let Some(version) = version {
              if let Some(q) = extra_query {
                extra_query = Some(q + "&v=" + version.as_str());
              } else {
                extra_query = Some("v=".to_owned() + version.as_str());
              }
            }
          }
          _ => {
            extra_query = Some("url".to_owned());
          }
        }
      }
    }

    if raw_query.as_ref().is_some() || extra_query.as_ref().is_some() {
      resolved_url += "?";
    }
    if let Some(raw_query) = raw_query.as_ref() {
      resolved_url += raw_query.join("&").as_str();
    }
    if let Some(extra_query) = extra_query.as_ref() {
      if raw_query.as_ref().is_some() {
        resolved_url += "&";
      }
      resolved_url += extra_query;
    }

    // update the dep graph
    self.deps.push((specifier.to_owned(), resolved_url.clone()));

    resolved_url
  }
}
