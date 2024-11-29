use std::collections::{HashMap, HashSet};

use crate::{EvalContext, Signal, SignalDef, SignalMatrix};

/// A planned evaluation of targets in a [`SignalMatrix`].
#[derive(Debug)]
pub struct PlannedEvaluation<'m, T: SignalDef> {
  matrix:       &'m SignalMatrix<T>,
  root_targets: HashSet<Signal>,
  passes:       Vec<EvaluationPassDescriptor>,
}

impl<'m, T: SignalDef> PlannedEvaluation<'m, T> {
  /// Create a new planned evaluation of the given root targets in the given
  /// [`SignalMatrix`].
  pub fn new(
    matrix: &'m SignalMatrix<T>,
    root_targets: HashSet<Signal>,
  ) -> Self {
    let dep_registry = matrix.defset.dependency_registry();

    let mut passes = vec![];
    // targets that must be satisfied in the current pass
    let mut unsatisfied_targets = root_targets.clone();

    loop {
      let pass = EvaluationPassDescriptor {
        targets: unsatisfied_targets.clone(),
      };

      unsatisfied_targets = unsatisfied_targets
        .iter()
        .flat_map(|target| dep_registry.get(target).unwrap())
        .cloned()
        .collect();

      if pass.targets.is_empty() {
        break;
      }

      passes.push(pass);
    }

    passes.reverse();

    // now go through passes start to finish and remove targets from later
    // passes if they are satisfied by an earlier pass

    let mut queued_targets: HashSet<Signal> = HashSet::new();
    for pass in passes.iter_mut() {
      pass.targets.retain(|target| {
        if queued_targets.contains(target) {
          false
        } else {
          queued_targets.insert(*target);
          true
        }
      });
    }

    PlannedEvaluation {
      matrix,
      root_targets,
      passes,
    }
  }

  /// Get all targets that are queued for evaluation in this planned evaluation.
  pub fn all_queued_targets(&self) -> HashSet<Signal> {
    self
      .passes
      .iter()
      .flat_map(|pass| pass.targets.iter())
      .cloned()
      .collect()
  }

  /// Run the planned evaluation, updating the given value map with the results.
  pub fn run(&self, values: &mut EvaluationValueMap<T>) {
    for (i, pass) in self.passes.iter().enumerate() {
      for target in pass.targets.iter() {
        let def = self.matrix.defset.get(*target).unwrap();
        let deps = def.dependencies();

        let context_values = deps.iter().map(|dep| {
          let value = values
            .values
            .get(dep)
            .and_then(|v| v.as_ref())
            .unwrap_or_else(|| {
              panic!(
                "Missing value for dependency {dep:?} while evaluating \
                 {target:?} in pass {i}"
              )
            });
          (*dep, value)
        });
        let context = EvalContext {
          values: context_values.collect(),
        };

        let value = T::evaluator()(&context, def);

        values.values.insert(*target, Some(value));
      }
    }
  }
}

/// Describes a pass in a planned evaluation.
#[derive(Debug)]
pub struct EvaluationPassDescriptor {
  targets: HashSet<Signal>,
}

/// A map of values for evaluated signals.
#[derive(Debug)]
pub struct EvaluationValueMap<T: SignalDef> {
  values: HashMap<Signal, Option<T::Value>>,
}

impl<T: SignalDef> EvaluationValueMap<T> {
  /// Create a new empty value map for the given targets.
  pub fn new_empty(targets: HashSet<Signal>) -> Self {
    EvaluationValueMap {
      values: targets.iter().map(|s| (*s, None)).collect(),
    }
  }

  /// Get the value for the given signal, if it has been evaluated.
  pub fn get(&self, signal: Signal) -> Option<&T::Value> {
    self.values.get(&signal).and_then(|v| v.as_ref())
  }
}
