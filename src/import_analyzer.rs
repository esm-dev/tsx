use crate::resolver::Resolver;
use crate::swc_helpers::*;
use std::cell::RefCell;
use std::rc::Rc;
use swc_ecmascript::ast::*;
use swc_ecmascript::visit::{Fold, FoldWith, noop_fold_type};

pub struct ImportAnalyzer {
  pub resolver: Rc<RefCell<Resolver>>,
}

impl Fold for ImportAnalyzer {
  noop_fold_type!();

  // resolve import/export url
  fn fold_module_items(&mut self, module_items: Vec<ModuleItem>) -> Vec<ModuleItem> {
    let mut items = Vec::<ModuleItem>::new();

    for item in module_items {
      match item {
        ModuleItem::ModuleDecl(decl) => {
          let item: ModuleItem = match decl {
            // match: import React, { useState } from "https://esm.sh/react"
            ModuleDecl::Import(import_decl) => {
              if import_decl.type_only {
                // ingore type import
                ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl))
              } else {
                let mut resolver = self.resolver.borrow_mut();
                let with_type = if let Some(with) = import_decl.with.as_ref() {
                  if let Some(Expr::Lit(Lit::Str(s))) = get_object_value(with, "type") {
                    Some(s.value.to_string())
                  } else {
                    None
                  }
                } else {
                  None
                };
                // remove `with { type: "rpc" }` from import declaration
                let with = if with_type.as_ref().is_none_or(|e| e != "rpc") {
                  import_decl.with
                } else {
                  None
                };
                let resolved_url = resolver.resolve(import_decl.src.value.as_ref(), with_type);
                ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                  src: Box::new(new_str(&resolved_url)),
                  with,
                  ..import_decl
                }))
              }
            }
            // match: export { default as React, useState } from "https://esm.sh/react"
            // match: export * as React from "https://esm.sh/react"
            ModuleDecl::ExportNamed(NamedExport {
              type_only,
              src: Some(src),
              specifiers,
              span,
              with,
            }) => {
              if type_only {
                // ingore type export
                ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(NamedExport {
                  span,
                  specifiers,
                  src: Some(src),
                  type_only,
                  with,
                }))
              } else {
                let mut resolver = self.resolver.borrow_mut();
                let resolved_url = resolver.resolve(src.value.as_ref(), None);
                ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(NamedExport {
                  span,
                  specifiers,
                  src: Some(Box::new(new_str(&resolved_url))),
                  type_only,
                  with,
                }))
              }
            }
            // match: export * from "https://esm.sh/react"
            ModuleDecl::ExportAll(export_all) => {
              let mut resolver = self.resolver.borrow_mut();
              let resolved_url = resolver.resolve(export_all.src.value.as_ref(), None);
              ModuleItem::ModuleDecl(ModuleDecl::ExportAll(ExportAll {
                src: Box::new(new_str(&resolved_url)),
                ..export_all
              }))
            }
            _ => ModuleItem::ModuleDecl(decl),
          };
          items.push(item.fold_children_with(self));
        }
        _ => {
          items.push(item.fold_children_with(self));
        }
      };
    }

    items
  }

  // resolve dynamic import url
  fn fold_call_expr(&mut self, mut call: CallExpr) -> CallExpr {
    if is_call_expr_by_name(&call, "import") {
      let src = match call.args.first() {
        Some(ExprOrSpread { expr, .. }) => match expr.as_ref() {
          Expr::Lit(Lit::Str(s)) => Some(s),
          _ => None,
        },
        _ => None,
      };
      if let Some(src) = src {
        let mut resolver = self.resolver.borrow_mut();
        let with_type = match call.args.get(1) {
          Some(ExprOrSpread { expr, .. }) => match expr.as_ref() {
            Expr::Object(obj) => match get_object_value(obj, "with") {
              Some(Expr::Object(obj)) => match get_object_value(obj, "type") {
                Some(Expr::Lit(Lit::Str(s))) => Some(s.value.to_string()),
                _ => None,
              },
              _ => None,
            },
            _ => None,
          },
          _ => None,
        };
        let new_src = resolver.resolve(src.value.as_ref(), with_type);
        call.args[0] = ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Lit(Lit::Str(new_str(&new_src)))),
        }
      }
    }
    call.fold_children_with(self)
  }
}
