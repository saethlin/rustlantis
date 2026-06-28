use std::borrow::BorrowMut;

use mir::{
    syntax::{Callee, Mutability, Operand, Place, TyId, TyKind},
    tyctxt::TyCtxt,
};
use rand::{Rng, seq::IteratorRandom};

use crate::{literal::GenLiteral, mem::BasicMemory, place_select::PlaceSelector};

use super::{GenerationCtx, Result, SelectionError};

pub trait CoreIntrinsic {
    fn name(&self) -> &'static str;

    fn dest_type(&self, ty: TyId, tcx: &TyCtxt) -> bool;

    fn choose_operands(&self, ctx: &GenerationCtx, dest: &Place) -> Option<Vec<Operand>>;

    fn generate_terminator(
        &self,
        ctx: &GenerationCtx,
        dest: &Place,
    ) -> Result<(Callee, Vec<Operand>)> {
        if !self.dest_type(dest.ty(ctx.current_decls(), &ctx.tcx), &ctx.tcx) {
            return Err(SelectionError::Exhausted);
        }
        let args = self
            .choose_operands(ctx, dest)
            .ok_or(SelectionError::Exhausted)?;
        Ok((Callee::Intrinsic(self.name()), args))
    }
}

/// A floating-point math intrinsic whose operands and destination all share
/// the same float type `ty`, taking `arity` operands, e.g. `sqrtf64` (arity 1),
/// `minimumf32` (arity 2), or `fmaf64` (arity 3).
///
/// Only deterministic operations are modelled here. Operations that Miri treats
/// as non-deterministic (the transcendental functions such as `sinf64`, the
/// `*_fast`/`*_algebraic` variants, `fmuladd`, and the sign-of-zero-ignoring
/// `*_nsz` min/max) are deliberately excluded, as non-determinism is
/// incompatible with difftesting. `copysign` is also excluded because it would
/// expose the non-deterministic sign bit of a computed NaN.
struct FloatMathIntrinsic {
    name: &'static str,
    ty: TyId,
    arity: usize,
}
impl CoreIntrinsic for FloatMathIntrinsic {
    fn name(&self) -> &'static str {
        self.name
    }

    fn dest_type(&self, ty: TyId, _: &TyCtxt) -> bool {
        ty == self.ty
    }

    fn choose_operands(&self, ctx: &GenerationCtx, dest: &Place) -> Option<Vec<Operand>> {
        let mut args = Vec::with_capacity(self.arity);
        for _ in 0..self.arity {
            args.push(ctx.choose_operand(&[self.ty], dest).ok()?);
        }
        Some(args)
    }
}

/// The set of deterministic floating-point math intrinsics, as
/// `(f32 name, f64 name, arity)`. Some are spelled with an `f32`/`f64` suffix
/// while others (e.g. `fabs`) are generic and rely on inference from the
/// operand types; both are emitted verbatim after `core::intrinsics::`.
const FLOAT_MATH_INTRINSICS: &[(&str, &str, usize)] = &[
    ("sqrtf32", "sqrtf64", 1),
    ("fabs", "fabs", 1),
    ("floorf32", "floorf64", 1),
    ("ceilf32", "ceilf64", 1),
    ("truncf32", "truncf64", 1),
    ("roundf32", "roundf64", 1),
    ("round_ties_even_f32", "round_ties_even_f64", 1),
    ("minimumf32", "minimumf64", 2),
    ("maximumf32", "maximumf64", 2),
    ("fmaf32", "fmaf64", 3),
];

pub(super) struct ArithOffset;
impl CoreIntrinsic for ArithOffset {
    fn name(&self) -> &'static str {
        "arith_offset"
    }

    fn dest_type(&self, ty: TyId, tcx: &TyCtxt) -> bool {
        matches!(ty.kind(tcx), TyKind::RawPtr(.., Mutability::Not))
    }

    fn choose_operands(&self, ctx: &GenerationCtx, dest: &Place) -> Option<Vec<Operand>> {
        let (ptrs, weights) = PlaceSelector::for_offsetee(ctx.tcx.clone())
            .of_ty(dest.ty(ctx.current_decls(), &ctx.tcx))
            .except(dest)
            .into_weighted(&ctx.pt)?;
        let ptr = ctx
            .make_choice_weighted(ptrs.into_iter(), weights, |ppath| {
                Ok(ppath.to_place(&ctx.pt))
            })
            .ok()?;

        let offset = ctx.pt.get_offset(&ptr);

        let mut rng = ctx.rng.borrow_mut();
        let new_offset = match offset {
            // Don't break roundtripped pointer
            Some(0) => {
                return None;
            }
            Some(existing) if existing.checked_neg().is_some() && rng.random_bool(0.5) => {
                Operand::Constant((-existing).try_into().unwrap())
            }
            _ => PlaceSelector::for_known_val(ctx.tcx.clone())
                .of_ty(TyCtxt::ISIZE)
                .into_iter_place(&ctx.pt)
                .choose(&mut *rng)
                .map(Operand::Copy)
                .unwrap_or_else(|| {
                    Operand::Constant(
                        rng.borrow_mut()
                            .gen_literal(TyCtxt::ISIZE, &ctx.tcx)
                            .expect("can generate a literal"),
                    )
                }),
        };

        Some(vec![Operand::Copy(ptr), new_offset])
    }
}

/// Every primitive integer type. Used to pick an operand of an arbitrary
/// integer type for intrinsics whose argument type is independent of (or merely
/// constrained relative to) the destination type.
const INT_TYS: [TyId; 12] = [
    TyCtxt::ISIZE,
    TyCtxt::I8,
    TyCtxt::I16,
    TyCtxt::I32,
    TyCtxt::I64,
    TyCtxt::I128,
    TyCtxt::USIZE,
    TyCtxt::U8,
    TyCtxt::U16,
    TyCtxt::U32,
    TyCtxt::U64,
    TyCtxt::U128,
];

/// An integer intrinsic of shape `fn(T) -> T` over any integer type, e.g.
/// `bswap` or `bitreverse`.
struct IntUnaryOp {
    name: &'static str,
}
impl CoreIntrinsic for IntUnaryOp {
    fn name(&self) -> &'static str {
        self.name
    }

    fn dest_type(&self, ty: TyId, tcx: &TyCtxt) -> bool {
        matches!(ty.kind(tcx), TyKind::Int(..) | TyKind::Uint(..))
    }

    fn choose_operands(&self, ctx: &GenerationCtx, dest: &Place) -> Option<Vec<Operand>> {
        let arg = ctx
            .choose_operand(&[dest.ty(ctx.current_decls(), &ctx.tcx)], dest)
            .ok()?;
        Some(vec![arg])
    }
}

/// An integer intrinsic of shape `fn(T, T) -> T` over any integer type, e.g.
/// `wrapping_add` or `saturating_sub`. Both operands share the destination
/// type.
struct IntBinOp {
    name: &'static str,
}
impl CoreIntrinsic for IntBinOp {
    fn name(&self) -> &'static str {
        self.name
    }

    fn dest_type(&self, ty: TyId, tcx: &TyCtxt) -> bool {
        matches!(ty.kind(tcx), TyKind::Int(..) | TyKind::Uint(..))
    }

    fn choose_operands(&self, ctx: &GenerationCtx, dest: &Place) -> Option<Vec<Operand>> {
        let ty = dest.ty(ctx.current_decls(), &ctx.tcx);
        let a = ctx.choose_operand(&[ty], dest).ok()?;
        let b = ctx.choose_operand(&[ty], dest).ok()?;
        Some(vec![a, b])
    }
}

/// A bit-counting intrinsic of shape `fn(T) -> u32`, e.g. `ctpop`, `ctlz`, or
/// `cttz`. The operand may be any integer type, independent of the `u32`
/// destination. The `_nonzero` variants are excluded as they are UB on zero.
struct IntBitCount {
    name: &'static str,
}
impl CoreIntrinsic for IntBitCount {
    fn name(&self) -> &'static str {
        self.name
    }

    fn dest_type(&self, ty: TyId, _: &TyCtxt) -> bool {
        ty == TyCtxt::U32
    }

    fn choose_operands(&self, ctx: &GenerationCtx, dest: &Place) -> Option<Vec<Operand>> {
        let arg = ctx.choose_operand(&INT_TYS, dest).ok()?;
        Some(vec![arg])
    }
}

/// A rotate intrinsic of shape `fn(T, u32) -> T` over any integer type, i.e.
/// `rotate_left` or `rotate_right`. The value shares the destination type; the
/// shift amount is always a `u32`.
struct IntRotate {
    name: &'static str,
}
impl CoreIntrinsic for IntRotate {
    fn name(&self) -> &'static str {
        self.name
    }

    fn dest_type(&self, ty: TyId, tcx: &TyCtxt) -> bool {
        matches!(ty.kind(tcx), TyKind::Int(..) | TyKind::Uint(..))
    }

    fn choose_operands(&self, ctx: &GenerationCtx, dest: &Place) -> Option<Vec<Operand>> {
        let value = ctx
            .choose_operand(&[dest.ty(ctx.current_decls(), &ctx.tcx)], dest)
            .ok()?;
        let shift = ctx.choose_operand(&[TyCtxt::U32], dest).ok()?;
        Some(vec![value, shift])
    }
}

pub(super) struct Transmute;
impl CoreIntrinsic for Transmute {
    fn name(&self) -> &'static str {
        "transmute"
    }

    fn dest_type(&self, ty: TyId, tcx: &TyCtxt) -> bool {
        if ty.contains(tcx, |tcx, ty| match ty.kind(tcx) {
            // Tys with value validity contstraints
            TyKind::Unit | TyKind::Bool | TyKind::Char | TyKind::RawPtr(..) | TyKind::Ref(..) => {
                true
            } // TODO: pointer transmute
            _ => false,
        }) {
            return false;
        }
        if BasicMemory::ty_size(ty, tcx).is_none() {
            return false;
        }
        true
    }

    fn choose_operands(&self, ctx: &GenerationCtx, dest: &Place) -> Option<Vec<Operand>> {
        let dest_size = BasicMemory::ty_size(dest.ty(ctx.current_decls(), &ctx.tcx), &ctx.tcx)
            .expect("dest must have known size");
        // Avoid pointer to int casts
        let allowed_tys: Vec<TyId> = ctx
            .tcx
            .indices()
            .filter(|ty| {
                !ty.contains(&ctx.tcx, |tcx, ty| {
                    // Avoid inspecting the bytes in fp as NaN payload is nd
                    ty.is_any_ptr(tcx) || ty == TyCtxt::F32 || ty == TyCtxt::F64
                })
            })
            .collect();

        let (srcs, weights) = PlaceSelector::for_argument(ctx.tcx.clone())
            .of_tys(&allowed_tys)
            .of_size(dest_size)
            .except(dest)
            .into_weighted(&ctx.pt)?;
        let src = ctx
            .make_choice_weighted(srcs.into_iter(), weights, |ppath| {
                Ok(ppath.to_place(&ctx.pt))
            })
            .ok()?;
        if src.ty(ctx.current_decls(), &ctx.tcx).is_copy(&ctx.tcx) {
            Some(vec![Operand::Copy(src)])
        } else {
            Some(vec![Operand::Move(src)])
        }
    }
}

impl GenerationCtx {
    pub fn choose_intrinsic(&self, dest: &Place) -> Result<(Callee, Vec<Operand>)> {
        let mut choices: Vec<Box<dyn CoreIntrinsic>> = vec![
            Box::new(ArithOffset),
            Box::new(Transmute),
        ];
        for &(f32_name, f64_name, arity) in FLOAT_MATH_INTRINSICS {
            choices.push(Box::new(FloatMathIntrinsic {
                name: f32_name,
                ty: TyCtxt::F32,
                arity,
            }));
            choices.push(Box::new(FloatMathIntrinsic {
                name: f64_name,
                ty: TyCtxt::F64,
                arity,
            }));
        }
        // Deterministic integer intrinsics. The `_nonzero` bit-counting
        // variants, `exact_div`, the `unchecked_*` family and `disjoint_bitor`
        // are excluded because they carry safety preconditions (UB if violated).
        for name in ["bswap", "bitreverse"] {
            choices.push(Box::new(IntUnaryOp { name }));
        }
        for name in ["ctpop", "ctlz", "cttz"] {
            choices.push(Box::new(IntBitCount { name }));
        }
        for name in [
            "wrapping_add",
            "wrapping_sub",
            "wrapping_mul",
            "saturating_add",
            "saturating_sub",
        ] {
            choices.push(Box::new(IntBinOp { name }));
        }
        for name in ["rotate_left", "rotate_right"] {
            choices.push(Box::new(IntRotate { name }));
        }

        let intrinsic = self.make_choice(choices.iter(), Result::Ok)?;
        intrinsic.generate_terminator(self, dest)
    }
}
