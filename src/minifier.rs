use swc_common::comments::{Comments, SingleThreadedComments};
use swc_common::sync::Lrc;
use swc_common::util::take::Take;
use swc_common::{Mark, SourceMap};
use swc_ecma_minifier::optimize;
use swc_ecma_minifier::option::{MangleOptions, MinifyOptions};
use swc_ecmascript::ast::*;
use swc_ecmascript::visit::{noop_visit_mut_type, VisitMut};

pub struct Minifier {
  pub cm: Lrc<SourceMap>,
  pub comments: Option<SingleThreadedComments>,
  pub unresolved_mark: Mark,
  pub top_level_mark: Mark,
  pub keep_names: bool,
}

impl VisitMut for Minifier {
  noop_visit_mut_type!();

  fn visit_mut_module(&mut self, m: &mut Module) {
    m.map_with_mut(|m| {
      optimize(
        m.into(),
        self.cm.clone(),
        self.comments.as_ref().map(|v| v as &dyn Comments),
        None,
        &MinifyOptions {
          compress: Some(Default::default()),
          mangle: Some(MangleOptions {
            top_level: Some(true),
            keep_class_names: self.keep_names,
            keep_fn_names: self.keep_names,
            keep_private_props: self.keep_names,
            ..Default::default()
          }),
          ..Default::default()
        },
        &swc_ecma_minifier::option::ExtraOptions {
          unresolved_mark: self.unresolved_mark,
          top_level_mark: self.top_level_mark,
        },
      )
      .expect_module()
    })
  }
}
