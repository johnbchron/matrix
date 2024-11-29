use std::collections::HashSet;

use crate::{Evaluator, Signal, SignalDef};

#[derive(Debug)]
pub enum FloatMapSignalDef {
  Constant(f64),
  UnaryOp(UnaryOp),
  BinaryOp(FloatBinaryOp),
}

#[derive(Debug)]
pub enum FloatBinaryOp {
  Add(Signal, Signal),
  Sub(Signal, Signal),
  Mul(Signal, Signal),
  Div(Signal, Signal),
  Pow(Signal, Signal),
}

#[derive(Debug)]
pub enum UnaryOp {
  Neg(Signal),
}

impl SignalDef for FloatMapSignalDef {
  type Value = f64;

  fn dependencies(&self) -> HashSet<Signal> {
    match self {
      FloatMapSignalDef::Constant(_) => HashSet::new(),
      FloatMapSignalDef::UnaryOp(op) => match op {
        UnaryOp::Neg(s) => vec![*s].into_iter().collect(),
      },
      FloatMapSignalDef::BinaryOp(op) => match op {
        FloatBinaryOp::Add(a, b) => vec![*a, *b].into_iter().collect(),
        FloatBinaryOp::Sub(a, b) => vec![*a, *b].into_iter().collect(),
        FloatBinaryOp::Mul(a, b) => vec![*a, *b].into_iter().collect(),
        FloatBinaryOp::Div(a, b) => vec![*a, *b].into_iter().collect(),
        FloatBinaryOp::Pow(a, b) => vec![*a, *b].into_iter().collect(),
      },
    }
  }

  fn evaluator() -> Evaluator<Self, Self::Value> {
    |ctx, def| match def {
      FloatMapSignalDef::Constant(value) => *value,
      FloatMapSignalDef::UnaryOp(op) => match op {
        UnaryOp::Neg(s) => -ctx.values[s],
      },
      FloatMapSignalDef::BinaryOp(op) => match op {
        FloatBinaryOp::Add(a, b) => ctx.values[a] + ctx.values[b],
        FloatBinaryOp::Sub(a, b) => ctx.values[a] - ctx.values[b],
        FloatBinaryOp::Mul(a, b) => ctx.values[a] * ctx.values[b],
        FloatBinaryOp::Div(a, b) => ctx.values[a] / ctx.values[b],
        FloatBinaryOp::Pow(a, b) => ctx.values[a].powf(*ctx.values[b]),
      },
    }
  }
}