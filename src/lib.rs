mod eval;
mod example_f64;

use std::{
  collections::{HashMap, HashSet},
  fmt::Debug,
};

pub use eval::*;
pub use example_f64::*;

#[derive(Debug, Default)]
pub struct SignalDefMap<T: SignalDef> {
  map:     HashMap<Signal, T>,
  last_id: u64,
}

impl<T: SignalDef> SignalDefMap<T> {
  pub fn new() -> Self {
    SignalDefMap {
      map:     HashMap::new(),
      last_id: 0,
    }
  }

  pub fn insert(&mut self, def: T) -> Signal {
    let id = Signal(self.last_id);
    self.last_id += 1;
    self.map.insert(id, def);
    id
  }

  pub fn get(&self, signal: Signal) -> Option<&T> { self.map.get(&signal) }

  fn dependency_registry(&self) -> HashMap<Signal, HashSet<Signal>> {
    self
      .map
      .iter()
      .map(|(signal, def)| (*signal, def.dependencies()))
      .collect()
  }
}

#[derive(Debug)]
pub struct SignalMatrix<T: SignalDef> {
  defset: SignalDefMap<T>,
}

impl<T: SignalDef> SignalMatrix<T> {
  pub fn new(defset: SignalDefMap<T>) -> Self { SignalMatrix { defset } }

  pub fn plan_evaluation(
    &self,
    root_targets: HashSet<Signal>,
  ) -> PlannedEvaluation<T> {
    PlannedEvaluation::new(self, root_targets)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signal(u64);

pub struct EvalContext<'c, T: SignalDef> {
  values: HashMap<Signal, &'c T::Value>,
}

pub type Evaluator<T, V> = fn(&EvalContext<T>, &T) -> V;

pub trait SignalDef: Debug + Sized {
  type Value: Sized;

  fn dependencies(&self) -> HashSet<Signal>;
  fn evaluator() -> Evaluator<Self, Self::Value>;
}
