use swc_common::{DUMMY_SP, SyntaxContext};
use swc_ecmascript::ast::*;
use swc_ecmascript::utils::quote_ident;

pub fn get_object_value<'a>(obj: &'a ObjectLit, key: &str) -> Option<&'a Expr> {
  obj.props.iter().find_map(|prop| match prop {
    PropOrSpread::Prop(kv) => {
      if let Prop::KeyValue(KeyValueProp {
        key: PropName::Ident(ident),
        value,
      }) = kv.as_ref()
      {
        if ident.sym.eq(key) {
          return Some(value.as_ref());
        }
      }
      None
    }
    _ => None,
  })
}

pub fn assign_decl(var_name: &str, expr: Expr) -> ModuleItem {
  ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    kind: VarDeclKind::Var,
    declare: false,
    decls: vec![VarDeclarator {
      span: DUMMY_SP,
      name: pat_id(var_name),
      init: Some(Box::new(expr)),
      definite: false,
    }],
  }))))
}

pub fn window_assign(name: &str, expr: Expr) -> ModuleItem {
  ModuleItem::Stmt(Stmt::Expr(ExprStmt {
    span: DUMMY_SP,
    expr: Box::new(Expr::Assign(AssignExpr {
      span: DUMMY_SP,
      op: AssignOp::Assign,
      left: AssignTarget::Simple(SimpleAssignTarget::Member(member_expr(ident_expr("window"), name))),
      right: Box::new(expr),
    })),
  }))
}

pub fn pat_id(id: &str) -> Pat {
  Pat::Ident(BindingIdent {
    id: quote_ident!(id).into(),
    type_ann: None,
  })
}

pub fn import_name(name: &str) -> ImportSpecifier {
  ImportSpecifier::Named(ImportNamedSpecifier {
    span: DUMMY_SP,
    local: quote_ident!(name).into(),
    imported: None,
    is_type_only: false,
  })
}

pub fn member_expr(obj: Expr, key: &str) -> MemberExpr {
  MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(obj),
    prop: MemberProp::Ident(quote_ident!(key)),
  }
}

pub fn simple_member_expr(obj: &str, key: &str) -> Expr {
  Expr::Member(MemberExpr {
    span: DUMMY_SP,
    obj: Box::new(ident_expr(obj)),
    prop: MemberProp::Ident(quote_ident!(key)),
  })
}

pub fn is_call_expr_by_name(call: &CallExpr, name: &str) -> bool {
  let callee = match &call.callee {
    Callee::Super(_) => return false,
    Callee::Import(_) => return name.eq("import"),
    Callee::Expr(callee) => callee.as_ref(),
  };

  match callee {
    Expr::Ident(id) => id.sym.as_ref().eq(name),
    _ => false,
  }
}

pub fn new_str(s: &str) -> Str {
  Str {
    span: DUMMY_SP,
    value: s.into(),
    raw: None,
  }
}

pub fn ident_expr(s: &str) -> Expr {
  Expr::Ident(quote_ident!(s).into())
}
