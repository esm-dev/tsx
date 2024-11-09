pub fn is_http_specifier(url: &str) -> bool {
  return url.starts_with("https://") || url.starts_with("http://");
}

pub fn is_relpath_specifier(specifier: &str) -> bool {
  return specifier.starts_with("./") || specifier.starts_with("../");
}

pub fn is_abspath_specifier(specifier: &str) -> bool {
  return specifier.starts_with("/") || specifier.starts_with("file://");
}
