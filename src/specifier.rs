use std::path::Path;
use std::str::FromStr;
use url::Url;

pub fn is_http_specifier(url: &str) -> bool {
  return url.starts_with("https://") || url.starts_with("http://");
}

pub fn is_relative_specifier(specifier: &str) -> bool {
  return specifier.starts_with("./") || specifier.starts_with("../");
}

pub fn is_css_specifier(url: &str) -> bool {
  if is_http_specifier(url) {
    return Url::from_str(url).unwrap().path().ends_with(".css");
  }
  return url.ends_with(".css");
}

pub fn has_file_extension(pathname: &str) -> bool {
  let basename = Path::new(pathname).file_name().unwrap().to_str().unwrap();
  return basename.contains(".") && !basename.starts_with(".");
}
