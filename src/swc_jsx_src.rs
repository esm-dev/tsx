// copied from https://github.com/swc-project/swc/blob/main/crates/swc_ecma_transforms_react/src/jsx_src/mod.rs
use swc_common::{sync::Lrc, SourceMap, DUMMY_SP};
use swc_ecmascript::ast::*;
use swc_ecmascript::utils::quote_ident;
use swc_ecmascript::visit::{noop_visit_mut_type, visit_mut_pass, VisitMut, VisitMutWith};

/// `@babel/plugin-transform-react-jsx-source`
pub fn jsx_src(cm: Lrc<SourceMap>, file_name: Option<String>) -> impl Pass {
  visit_mut_pass(JsxSrc { cm, file_name })
}

#[derive(Clone)]
struct JsxSrc {
  cm: Lrc<SourceMap>,
  file_name: Option<String>,
}

impl VisitMut for JsxSrc {
  noop_visit_mut_type!();

  fn visit_mut_jsx_opening_element(&mut self, e: &mut JSXOpeningElement) {
    if e.span == DUMMY_SP {
      return;
    }

    e.visit_mut_children_with(self);

    let loc = self.cm.lookup_char_pos(e.span.lo);
    let file_name = self.file_name.clone().unwrap_or(loc.file.name.to_string());

    e.attrs.push(JSXAttrOrSpread::JSXAttr(JSXAttr {
      span: DUMMY_SP,
      name: JSXAttrName::Ident(quote_ident!("__source")),
      value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
        span: DUMMY_SP,
        expr: JSXExpr::Expr(Box::new(
          ObjectLit {
            span: DUMMY_SP,
            props: vec![
              PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(quote_ident!("fileName")),
                value: Box::new(Expr::Lit(Lit::Str(Str {
                  span: DUMMY_SP,
                  raw: None,
                  value: file_name.into(),
                }))),
              }))),
              PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(quote_ident!("lineNumber")),
                value: loc.line.into(),
              }))),
              PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(quote_ident!("columnNumber")),
                value: (loc.col.0 + 1).into(),
              }))),
            ],
          }
          .into(),
        )),
      })),
    }));
  }
}
