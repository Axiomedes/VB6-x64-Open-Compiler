//! Type coercion and binary operator promotion rules according to VB6 semantics.

use crate::ast::expr::BinaryOperator;
use crate::types::VbType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoercionKind {
    Exact,
    Widening,
    Narrowing,
    Dynamic, // Involves Variant
    Incompatible,
}

/// Check the coercion relationship between source type and target type.
pub fn check_coercion(from: &VbType, target: &VbType) -> CoercionKind {
    if from == target {
        return CoercionKind::Exact;
    }

    if *from == VbType::Variant || *target == VbType::Variant {
        return CoercionKind::Dynamic;
    }

    // Both are numeric
    if from.is_numeric() && target.is_numeric() {
        let from_rank = numeric_rank(from);
        let target_rank = numeric_rank(target);
        if from_rank <= target_rank {
            return CoercionKind::Widening;
        } else {
            return CoercionKind::Narrowing;
        }
    }

    // Boolean <-> Numeric
    if *from == VbType::Boolean && target.is_numeric() {
        return CoercionKind::Widening;
    }
    if from.is_numeric() && *target == VbType::Boolean {
        return CoercionKind::Narrowing;
    }

    // String <-> FixedString
    if matches!((from, target), (VbType::String, VbType::FixedString(_)) | (VbType::FixedString(_), VbType::String)) {
        return CoercionKind::Exact;
    }

    // Array of same element type
    if let (VbType::Array(elem_from), VbType::Array(elem_to)) = (from, target) {
        if elem_from == elem_to {
            return CoercionKind::Exact;
        }
    }

    // Date <-> Double (Date is an IEEE 64-bit float internally)
    if (*from == VbType::Date && *target == VbType::Double) || (*from == VbType::Double && *target == VbType::Date) {
        return CoercionKind::Exact;
    }

    // Numeric / Boolean / Date -> String (implicit string conversion)
    if (from.is_numeric() || *from == VbType::Boolean || *from == VbType::Date) && *target == VbType::String {
        return CoercionKind::Widening;
    }

    CoercionKind::Incompatible
}

/// Rank numeric types for arithmetic promotion.
fn numeric_rank(ty: &VbType) -> u8 {
    match ty {
        VbType::Byte => 1,
        VbType::Integer => 2,
        VbType::Long => 3,
        VbType::LongPtr => 4,
        VbType::LongLong => 5,
        VbType::Single => 6,
        VbType::Double => 7,
        VbType::Currency => 8,
        _ => 0,
    }
}

/// Determine the resulting VbType when applying a binary operator to two operands.
pub fn promote_binary_operator(lhs: &VbType, op: &BinaryOperator, rhs: &VbType) -> Result<VbType, String> {
    // String concatenation operator '&' always yields String
    if *op == BinaryOperator::Concat {
        return Ok(VbType::String);
    }

    // Comparison operators always return Boolean
    if matches!(
        op,
        BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::LessThan
            | BinaryOperator::LessThanOrEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanOrEqual
            | BinaryOperator::Is
            | BinaryOperator::Like
    ) {
        return Ok(VbType::Boolean);
    }

    // If either operand is Variant, result is Variant at compile-time (resolved dynamically at runtime)
    if *lhs == VbType::Variant || *rhs == VbType::Variant {
        return Ok(VbType::Variant);
    }

    // String addition '+'
    if *op == BinaryOperator::Add && (*lhs == VbType::String || *rhs == VbType::String) {
        return Ok(VbType::String);
    }

    // Logical / Bitwise operators: And, Or, Xor, Eqv, Imp
    if matches!(
        op,
        BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Xor | BinaryOperator::Eqv | BinaryOperator::Imp
    ) {
        if *lhs == VbType::Boolean && *rhs == VbType::Boolean {
            return Ok(VbType::Boolean);
        }
        // Bitwise on integers: widen to larger integer type
        if lhs.is_integer() && rhs.is_integer() {
            let rank_l = numeric_rank(lhs);
            let rank_r = numeric_rank(rhs);
            return Ok(if rank_l >= rank_r { lhs.clone() } else { rhs.clone() });
        }
        return Ok(VbType::Long);
    }

    // Floating division '/' always returns Double, unless both are Single
    if *op == BinaryOperator::Divide {
        if *lhs == VbType::Single && *rhs == VbType::Single {
            return Ok(VbType::Single);
        }
        return Ok(VbType::Double);
    }

    // Integer division '\' always returns Long (or LongLong if 64-bit operand)
    if *op == BinaryOperator::IntegerDivide {
        if *lhs == VbType::LongLong || *rhs == VbType::LongLong {
            return Ok(VbType::LongLong);
        }
        return Ok(VbType::Long);
    }

    // Exponentiation '^' always returns Double
    if *op == BinaryOperator::Exponent {
        return Ok(VbType::Double);
    }

    // Modulo 'Mod' returns integer type (Long or Integer)
    if *op == BinaryOperator::Modulo {
        if *lhs == VbType::LongLong || *rhs == VbType::LongLong {
            return Ok(VbType::LongLong);
        }
        if *lhs == VbType::Long || *rhs == VbType::Long {
            return Ok(VbType::Long);
        }
        return Ok(VbType::Integer);
    }

    // Arithmetic operators: Add, Subtract, Multiply
    if matches!(op, BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply) {
        // Date arithmetic: Date + Double -> Date, Date - Date -> Double
        if *lhs == VbType::Date && *rhs == VbType::Date && *op == BinaryOperator::Subtract {
            return Ok(VbType::Double);
        }
        if (*lhs == VbType::Date && rhs.is_numeric()) || (lhs.is_numeric() && *rhs == VbType::Date) {
            return Ok(VbType::Date);
        }

        // Byte + Byte in VB6: arithmetic is promoted to Integer to avoid 8-bit overflow
        if *lhs == VbType::Byte && *rhs == VbType::Byte {
            return Ok(VbType::Integer);
        }

        if lhs.is_numeric() && rhs.is_numeric() {
            let rank_l = numeric_rank(lhs);
            let rank_r = numeric_rank(rhs);
            return Ok(if rank_l >= rank_r { lhs.clone() } else { rhs.clone() });
        }
    }

    Err(format!(
        "Operator '{:?}' not supported between types {:?} and {:?}",
        op, lhs, rhs
    ))
}
