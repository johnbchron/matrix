mod eval;
mod example_f64;

use std::{
  collections::{HashMap, HashSet},
  fmt::Debug,
};

pub use eval::*;
pub use example_f64::*;
use tracing::instrument;

/// A map of signal definitions.
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

  #[instrument]
  fn dependency_registry(&self) -> HashMap<Signal, HashSet<Signal>> {
    self
      .map
      .iter()
      .map(|(signal, def)| (*signal, def.dependencies()))
      .collect()
  }
}

/// The core graph type. Contains a signal map.
#[derive(Debug)]
pub struct SignalMatrix<T: SignalDef> {
  defset: SignalDefMap<T>,
}

impl<T: SignalDef> SignalMatrix<T> {
  /// Create a new signal matrix with the given signal definition map.
  pub fn new(defset: SignalDefMap<T>) -> Self { SignalMatrix { defset } }

  /// Build a [`PlannedEvaluation`] of the given root targets.
  pub fn plan_evaluation<P: EvaluationPlanner>(
    &self,
    root_targets: HashSet<Signal>,
  ) -> PlannedEvaluation<T> {
    PlannedEvaluation::new::<P>(self, root_targets)
  }
}

/// A handle to a signal in the graph. This is an ID for a signal definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Signal(u64);

/// Context given to an evaluator function. For providing dependencies.
pub struct EvalContext<'c, T: SignalDef> {
  values: HashMap<Signal, &'c T::Value>,
}

/// Trait for signal definitions.
pub trait SignalDef: Debug + Sync + Sized {
  /// The type of value that this signal definition evaluates to.
  type Value: Debug + Send + Sync + Sized;

  /// Get the dependencies of this signal definition.
  fn dependencies(&self) -> HashSet<Signal>;
  /// Evaluate this signal definition with the given context.
  fn evaluate(&self, ctx: &EvalContext<Self>) -> Self::Value;
}
