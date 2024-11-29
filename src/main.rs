use matrix::{
  EvaluationValueMap, FloatBinaryOp, FloatMapSignalDef, SignalDefMap,
  SignalMatrix, UnaryOp,
};

fn main() {
  let mut defset = SignalDefMap::new();

  let a = defset.insert(FloatMapSignalDef::Constant(1.0));
  let b = defset.insert(FloatMapSignalDef::Constant(2.0));
  let c = defset.insert(FloatMapSignalDef::BinaryOp(FloatBinaryOp::Add(a, b)));
  let d = defset.insert(FloatMapSignalDef::UnaryOp(UnaryOp::Neg(c)));
  let e = defset.insert(FloatMapSignalDef::BinaryOp(FloatBinaryOp::Mul(c, d)));

  let matrix = SignalMatrix::new(defset);

  let root_targets = vec![c, e].into_iter().collect();
  let planned_eval = matrix.plan_evaluation(root_targets);

  dbg!(&planned_eval);
  dbg!(&planned_eval.all_queued_targets());

  let mut values =
    EvaluationValueMap::new_empty(planned_eval.all_queued_targets());
  dbg!(&values);

  planned_eval.run(&mut values);

  dbg!(&values);

  println!("final answer: {}", values.get(e).unwrap());
}
