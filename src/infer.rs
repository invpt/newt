use std::{
    collections::{hash_map::Entry, HashMap},
    marker::PhantomData,
};

use crate::{
    diagnostics::Diagnostics,
    parse::{Expr, ExprKind, Literal, Symbol, SymbolId, Termination, Ty, UnknownTyId, Wildcard},
};

pub struct Inferrer<'d, 's> {
    phantom: PhantomData<&'s ()>,
    diag: Diagnostics<'d>,
    sym_types: HashMap<SymbolId, Ty>,
    parent: HashMap<UnknownTyId, Ty>,
}

impl<'d, 's> Inferrer<'d, 's> {
    pub fn infer(diag: Diagnostics<'d>, expr: &mut Expr<'s>) {}

    fn generate_constraints<'a>(&mut self, expr: &'a mut Expr<'s>) -> &'a Ty {
        match &mut expr.kind {
            ExprKind::Dict(_) => todo!(),
            ExprKind::Func(arg, ret, body) => {
                let t_arg = if let Some(arg) = arg {
                    Some(self.generate_constraints(arg))
                } else {
                    None
                };

                if let Some(ret) = ret {
                    todo!("ret type annotations")
                }

                let t_body = self.generate_constraints(body);

                self.equate(
                    expr.ty.clone(),
                    Ty::Func(t_arg.cloned().map(Box::new), Box::new(t_body.clone())),
                );
            }
            ExprKind::FuncSig(arg, ret) => {}
            ExprKind::If(cond, then, otherwise) => {
                let t_cond = self.generate_constraints(&mut *cond);
                self.equate(t_cond.clone(), Ty::Bool);
                let t_then = self.generate_constraints(&mut *then);
                if let Some(otherwise) = otherwise {
                    let t_otherwise = self.generate_constraints(&mut *otherwise);
                    self.equate(t_then.clone(), t_otherwise.clone());
                }
            }
            ExprKind::Tup(exprs) => {
                let mut tys = Vec::new();
                for expr in exprs.iter_mut() {
                    tys.push(self.generate_constraints(expr).clone());
                }
                self.equate(expr.ty.clone(), Ty::Tuple(tys.into_boxed_slice()));
            }
            ExprKind::Seq(exprs, termination) => {
                for expr in exprs.iter_mut() {
                    self.generate_constraints(expr);
                }
                match termination {
                    Termination::Terminated => {
                        self.equate(expr.ty.clone(), Ty::Tuple(Box::new([])));
                    }
                    Termination::Unterminated => {
                        if let Some(last) = exprs.last_mut() {
                            self.equate(expr.ty.clone(), last.ty.clone());
                        }
                    }
                }
            }
            ExprKind::EqAssert(a, b) => {
                let t_a = self.generate_constraints(a);
                let t_b = self.generate_constraints(b);
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
                let t_f = self.generate_constraints(&mut *f);
                let t_a = self.generate_constraints(&mut *a);
                self.equate(
                    t_f.clone(),
                    Ty::Func(Box::new(t_a.clone()), Box::new(expr.ty.clone())),
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
            todo!("decomposition")
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
            _ => todo!(),
        }
    }

    fn expr<'a>(&mut self, expr: &'a mut Expr<'s>, superty: &Ty) -> &'a Ty {
        let ty = match &mut expr.kind {
            ExprKind::Dict(_) => todo!(),
            ExprKind::Func(expr, expr1, expr2) => todo!(),
            ExprKind::If(expr, expr1, expr2) => todo!(),
            ExprKind::Tup(_) => todo!(),
            ExprKind::Seq(_, termination) => todo!(),
            ExprKind::EqAssert(expr, expr1) => todo!(),
            ExprKind::Wildcard(wildcard, symbol, expr) => todo!(),
            ExprKind::Bin(bin_op, expr, expr1) => todo!(),
            ExprKind::Apply(func, arg) => {
                todo!()
            }
            ExprKind::Name(symbol) => {
                todo!()
            }
            ExprKind::Literal(literal) => match literal {
                Literal::Integer(_) => Ty::Integer,
                Literal::String(_) => Ty::String,
            },
            ExprKind::Hole => todo!(),
        };

        &expr.ty
    }
}
