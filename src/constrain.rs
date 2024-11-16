use std::{
    collections::{hash_map::Entry, HashMap},
    marker::PhantomData,
};

use crate::{
    diagnostics::Diagnostics,
    parse::{Expr, ExprKind, Literal, Symbol, SymbolId, Termination, Ty, UnknownTyId, Wildcard},
};

pub struct Constrainer<'d, 's> {
    phantom: PhantomData<&'s ()>,
    diag: Diagnostics<'d>,
    sym_types: HashMap<SymbolId, Ty>,
    parent: HashMap<UnknownTyId, Ty>,
}

impl<'d, 's> Constrainer<'d, 's> {
    pub fn constrain(diag: Diagnostics<'d>, expr: &Expr<'s>) {}

    fn visit<'a>(&mut self, expr: &'a Expr<'s>) -> &'a Ty {
        match &expr.kind {
            ExprKind::Dict(_) => todo!(),
            ExprKind::Func(arg, ret, body) => {
                let t_arg = if let Some(arg) = arg {
                    Some(self.visit(arg))
                } else {
                    None
                };

                if let Some(ret) = ret {
                    todo!("ret type annotations")
                }

                let t_body = self.visit(body);

                self.equate(
                    expr.ty.clone(),
                    Ty::Func(t_arg.cloned().map(Box::new), Box::new(t_body.clone())),
                );
            }
            ExprKind::FuncSig(arg, ret) => {}
            ExprKind::If(cond, then, otherwise) => {
                let t_cond = self.visit(&*cond);
                self.equate(t_cond.clone(), Ty::Bool);
                let t_then = self.visit(&*then);
                if let Some(otherwise) = otherwise {
                    let t_otherwise = self.visit(&*otherwise);
                    self.equate(t_then.clone(), t_otherwise.clone());
                }
            }
            ExprKind::Tuple(exprs) => {
                let mut tys = Vec::new();
                for expr in exprs.iter() {
                    tys.push(self.visit(expr).clone());
                }
                self.equate(expr.ty.clone(), Ty::Tuple(tys.into_boxed_slice()));
            }
            ExprKind::Seq(exprs, termination) => {
                for expr in exprs.iter() {
                    self.visit(expr);
                }
                match termination {
                    Termination::Terminated => {
                        self.equate(expr.ty.clone(), Ty::Tuple(Box::new([])));
                    }
                    Termination::Unterminated => {
                        if let Some(last) = exprs.last() {
                            self.equate(expr.ty.clone(), last.ty.clone());
                        }
                    }
                }
            }
            ExprKind::EqAssert(a, b) => {
                let t_a = self.visit(a);
                let t_b = self.visit(b);
                self.equate(t_a.clone(), t_b.clone());
                self.equate(expr.ty.clone(), Ty::Tuple(Box::new([])));
            }
            ExprKind::Wildcard(
                wildcard,
                Symbol {
                    id: Some(symbol_id),
                    ..
                },
                annotation,
            ) => {
                match wildcard {
                    Wildcard::Set => self.lookup(symbol_id.clone(), &expr.ty),
                    Wildcard::Val | Wildcard::Var => (),
                }
                if let Some(_annotation) = annotation {
                    todo!("Support annotations")
                }
            }
            ExprKind::Wildcard(_, Symbol { id: None, .. }, _) => {
                // Can't do anything about an unresolved symbol
            }
            ExprKind::Bin(bin_op, expr, expr1) => todo!(),
            ExprKind::Apply(f, a) => {
                let t_f = self.visit(&*f);
                let t_a = self.visit(&*a);
                self.equate(
                    t_f.clone(),
                    Ty::Func(Some(Box::new(t_a.clone())), Box::new(expr.ty.clone())),
                );
            }
            ExprKind::Name(Symbol {
                id: Some(symbol_id),
                ..
            }) => self.lookup(*symbol_id, &expr.ty),
            ExprKind::Name(Symbol { id: None, .. }) => {
                // Nothing we can do for an unresolved symbol
            }
            ExprKind::Literal(literal) => match literal {
                Literal::Integer(_) => self.equate(expr.ty.clone(), Ty::Integer),
                Literal::String(_) => self.equate(expr.ty.clone(), Ty::String),
            },
            ExprKind::Hole => todo!(),
        }

        &expr.ty
    }

    fn equate(&mut self, t1: Ty, t2: Ty) {
        if let Ty::Unknown(id) = &t1 {
            self.parent.insert(id.clone(), t2);
        } else if let Ty::Unknown(id) = t2 {
            self.parent.insert(id, t1);
        } else {
            todo!("decomposition {t1:?}, {t2:?}")
        }
    }

    fn lookup(&mut self, symbol: SymbolId, current: &Ty) {
        match self.sym_types.entry(symbol) {
            Entry::Occupied(occupied_entry) => {
                let looked_up = occupied_entry.get().clone();
                self.equate(current.clone(), looked_up);
            }
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(current.clone());
                self.new_set(current.clone());
            }
        }
    }

    fn new_set(&mut self, ty: Ty) {
        match &ty {
            Ty::Integer | Ty::String => {
                // Primitives require no action
            }
            Ty::Unknown(unknown_ty_id) => {
                self.parent.insert(unknown_ty_id.clone(), ty);
            }
            _ => todo!("new_set {ty:?}"),
        }
    }
}
