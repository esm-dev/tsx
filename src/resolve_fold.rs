use crate::resolver::Resolver;
use crate::swc_helpers::{is_call_expr_by_name, new_str};
use std::{cell::RefCell, rc::Rc};
use swc_common::{Span};
use swc_ecmascript::ast::*;
use swc_ecmascript::visit::{noop_fold_type, Fold, FoldWith};

pub fn resolve_fold(
  resolver: Rc<RefCell<Resolver>>,
  mark_import_src_location: bool,
) -> impl Fold {
  ResolveFold {
    resolver,
    mark_import_src_location,
  }
}

pub struct ResolveFold {
  resolver: Rc<RefCell<Resolver>>,
  mark_import_src_location: bool,
}

impl Fold for ResolveFold {
  noop_fold_type!();

  // fold&resolve import/export url
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
                let resolved_url = resolver.resolve(
                  import_decl.src.value.as_ref(),
                  false,
                  mark_span(&import_decl.src.span, self.mark_import_src_location),
                );
                ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                  src: Box::new(new_str(&resolved_url)),
                  ..import_decl
                }))
              }
            }
            // match: export { default as React, useState } from "https://esm.sh/react"
            // match: export * as React from "https://esm.sh/react"
            ModuleDecl::ExportNamed(NamedExport {
              type_only,
              specifiers,
              src: Some(src),
              span,
              asserts,
            }) => {
              if type_only {
                // ingore type export
                ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(NamedExport {
                  span,
                  specifiers,
                  src: Some(src),
                  type_only,
                  asserts,
                }))
              } else {
                let mut resolver = self.resolver.borrow_mut();
                let resolved_url = resolver.resolve(
                  src.value.as_ref(),
                  false,
                  mark_span(&src.span, self.mark_import_src_location),
                );
                ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(NamedExport {
                  span,
                  specifiers,
                  src: Some(Box::new(new_str(&resolved_url))),
                  type_only,
                  asserts,
                }))
              }
            }
            // match: export * from "https://esm.sh/react"
            ModuleDecl::ExportAll(ExportAll { src, span, asserts }) => {
              let mut resolver = self.resolver.borrow_mut();
              let resolved_url = resolver.resolve(
                src.value.as_ref(),
                false,
                mark_span(&src.span, self.mark_import_src_location),
              );
              ModuleItem::ModuleDecl(ModuleDecl::ExportAll(ExportAll {
                span,
                src: Box::new(new_str(&resolved_url)),
                asserts,
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

  // resolve worker import url
  fn fold_new_expr(&mut self, mut new_expr: NewExpr) -> NewExpr {
    let ok = match new_expr.callee.as_ref() {
      Expr::Ident(id) => id.sym.as_ref().eq("Worker"),
      _ => false,
    };
    if ok {
      if let Some(args) = &mut new_expr.args {
        let src = match args.first() {
          Some(ExprOrSpread { expr, .. }) => match expr.as_ref() {
            Expr::Lit(lit) => match lit {
              Lit::Str(s) => Some(s),
              _ => None,
            },
            _ => None,
          },
          _ => None,
        };
        if let Some(src) = src {
          let mut resolver = self.resolver.borrow_mut();
          let new_src = resolver.resolve(
            src.value.as_ref(),
            true,
            mark_span(&src.span, self.mark_import_src_location),
          );

          args[0] = ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Lit(Lit::Str(new_str(&new_src)))),
          }
        }
      }
    };

    new_expr.fold_children_with(self)
  }

  // resolve dynamic import url
  fn fold_call_expr(&mut self, mut call: CallExpr) -> CallExpr {
    if is_call_expr_by_name(&call, "import") {
      let src = match call.args.first() {
        Some(ExprOrSpread { expr, .. }) => match expr.as_ref() {
          Expr::Lit(lit) => match lit {
            Lit::Str(s) => Some(s),
            _ => None,
          },
          _ => None,
        },
        _ => None,
      };
      if let Some(src) = src {
        let mut resolver = self.resolver.borrow_mut();
        let new_src = resolver.resolve(
          src.value.as_ref(),
          true,
          mark_span(&src.span, self.mark_import_src_location),
        );

        call.args[0] = ExprOrSpread {
          spread: None,
          expr: Box::new(Expr::Lit(Lit::Str(new_str(&new_src)))),
        }
      }
    }

    call.fold_children_with(self)
  }
}

fn mark_span(span: &Span, ok: bool) -> Option<Span> {
  if ok {
    Some(span.clone())
  } else {
    None
  }
}
