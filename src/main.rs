use matrix::{
  EvaluationValueMap, FloatBinaryOp, FloatMapSignalDef, Signal, SignalDefMap,
  SignalMatrix, UnaryOp,
};
use tracing_chrome::ChromeLayerBuilder;
use tracing_subscriber::{prelude::*, registry::Registry};

fn main() {
  let (chrome_layer, _guard) = ChromeLayerBuilder::new().build();
  tracing_subscriber::registry().with(chrome_layer).init();

  let mut defset = SignalDefMap::new();

  // let a = defset.insert(FloatMapSignalDef::Constant(1.0));
  // let b = defset.insert(FloatMapSignalDef::Constant(2.0));
  // let c = defset.insert(FloatMapSignalDef::BinaryOp(FloatBinaryOp::Add(a,
  // b))); let d = defset.insert(FloatMapSignalDef::UnaryOp(UnaryOp::Neg(c)));
  // let e = defset.insert(FloatMapSignalDef::BinaryOp(FloatBinaryOp::Mul(c,
  // d)));

  // create a symetric binary tree of addition operations
  let depth: i32 = 100;
  let mut last_level = (0..depth.pow(2))
    .map(|i| defset.insert(FloatMapSignalDef::Constant(i as f64)))
    .collect::<Vec<_>>();

  for _ in 0..depth {
    let mut new_level = Vec::new();
    for (i, a) in last_level.iter().step_by(2).enumerate() {
      let b = last_level.get(i * 2 + 1).unwrap_or(a);
      new_level.push(
        defset.insert(FloatMapSignalDef::BinaryOp(FloatBinaryOp::Add(*a, *b))),
      );
    }
    last_level = new_level;
  }
  let root = last_level[0];

  let root_targets = vec![root].into_iter().collect();
  let matrix = SignalMatrix::new(defset);

  let now = std::time::Instant::now();
  let planned_eval = matrix.plan_evaluation(root_targets);
  println!("planning took {:?}", now.elapsed());

  println!(
    "first pass size: {}",
    planned_eval.passes().first().unwrap().targets().len()
  );
  // println!("passes:");
  // planned_eval
  //   .passes()
  //   .iter()
  //   .enumerate()
  //   .for_each(|(i, pass)| {
  //     println!("  {i}:\t{:?}", pass);
  //   });

  let values = EvaluationValueMap::new_empty(planned_eval.all_queued_targets());

  let now = std::time::Instant::now();
  let values = planned_eval.run(values);
  println!("evaluation took {:?}", now.elapsed());

  // dbg!(&values);

  println!("final answer: {}", values.get(root).unwrap());
}
