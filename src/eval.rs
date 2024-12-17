use std::collections::{HashMap, HashSet};

use rayon::prelude::*;
use tracing::instrument;

use crate::{EvalContext, Signal, SignalDef, SignalMatrix};

pub trait EvaluationPlanner {
  fn plan_evaluation<Def: SignalDef>(
    matrix: &SignalMatrix<Def>,
    root_targets: HashSet<Signal>,
  ) -> PlannedEvaluation<'_, Def>;
}

pub struct CustomPlanner;

impl EvaluationPlanner for CustomPlanner {
  fn plan_evaluation<Def: SignalDef>(
    matrix: &SignalMatrix<Def>,
    root_targets: HashSet<Signal>,
  ) -> PlannedEvaluation<'_, Def> {
    let dep_registry = matrix.defset.dependency_registry();

    let mut passes = vec![];
    // targets that must be satisfied in the current pass
    let mut unsatisfied_targets = root_targets.clone();

    loop {
      let loop_span =
        tracing::info_span!("planning_pass", ?unsatisfied_targets);
      let _enter = loop_span.enter();
      let pass = EvaluationPassDescriptor {
        targets: unsatisfied_targets.clone(),
      };

      tracing::info_span!("unsatisfied_target_deps").in_scope(|| {
        unsatisfied_targets = unsatisfied_targets
          .iter()
          .flat_map(|target| dep_registry.get(target).unwrap())
          .cloned()
          .collect();
      });

      if pass.targets.is_empty() {
        break;
      }

      passes.push(pass);
    }

    tracing::info_span!("reverse_passes").in_scope(|| {
      passes.reverse();
    });

    // now go through passes start to finish and remove targets from later
    // passes if they are satisfied by an earlier pass

    tracing::info_span!("dedup_passes").in_scope(|| {
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
    });

    PlannedEvaluation {
      matrix,
      root_targets,
      passes,
    }
  }
}

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
  #[instrument]
  pub fn new<P: EvaluationPlanner>(
    matrix: &'m SignalMatrix<T>,
    root_targets: HashSet<Signal>,
  ) -> Self {
    P::plan_evaluation(matrix, root_targets)
  }

  /// Get all targets that are queued for evaluation in this planned evaluation.
  pub fn all_queued_targets(&self) -> HashSet<Signal> {
    self
      .passes
      .par_iter()
      .flat_map(|pass| pass.targets.par_iter())
      .copied()
      .collect()
  }

  /// Run the planned evaluation, updating the given value map with the results.
  #[instrument]
  pub fn run(
    &self,
    mut values: EvaluationValueMap<T>,
  ) -> EvaluationValueMap<T> {
    for (i, pass) in self.passes.iter().enumerate() {
      let pass_span = tracing::info_span!("evaluation_pass", i);
      let _enter = pass_span.enter();
      let evaluations: Vec<_> = pass
        .targets
        .par_iter()
        .map(|target| {
          let def = self.matrix.defset.get(*target).unwrap();
          let deps = def.dependencies();

          let context_gathering_span =
            tracing::info_span!("gather_context", ?deps);
          let _enter = context_gathering_span.enter();
          let context_values = deps.into_iter().map(|dep| {
            let value = values
              .values
              .get(&dep)
              .and_then(|v| v.as_ref())
              .unwrap_or_else(|| {
                panic!(
                  "Missing value for dependency {dep:?} while evaluating \
                   {target:?} in pass {i}"
                )
              });
            (dep, value)
          });
          let context = EvalContext {
            values: context_values.collect(),
          };
          drop(_enter);

          let evaluator_span = tracing::info_span!("evaluate");
          let _enter = evaluator_span.enter();
          let value = def.evaluate(&context);
          drop(_enter);

          (*target, value)
        })
        .collect();

      for (target, value) in evaluations {
        values.values.insert(target, Some(value));
      }
    }

    values
  }

  pub fn passes(&self) -> &[EvaluationPassDescriptor] { &self.passes }
}

/// Describes a pass in a planned evaluation.
#[derive(Debug)]
pub struct EvaluationPassDescriptor {
  targets: HashSet<Signal>,
}

impl EvaluationPassDescriptor {
  /// Get the targets in this pass.
  pub fn targets(&self) -> &HashSet<Signal> { &self.targets }
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
      values: targets.par_iter().map(|s| (*s, None)).collect(),
    }
  }

  /// Get the value for the given signal, if it has been evaluated.
  pub fn get(&self, signal: Signal) -> Option<&T::Value> {
    self.values.get(&signal).and_then(|v| v.as_ref())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{FloatBinaryOp, FloatMapSignalDef, SignalDefMap, UnaryOp};

  #[test]
  fn test_planned_evaluation() {
    let mut defset = SignalDefMap::new();

    let a = defset.insert(FloatMapSignalDef::Constant(1.0));
    let b = defset.insert(FloatMapSignalDef::Constant(2.0));
    let c =
      defset.insert(FloatMapSignalDef::BinaryOp(FloatBinaryOp::Add(a, b)));
    let d = defset.insert(FloatMapSignalDef::UnaryOp(UnaryOp::Neg(c)));
    let e =
      defset.insert(FloatMapSignalDef::BinaryOp(FloatBinaryOp::Mul(c, d)));

    let root_targets = vec![e].into_iter().collect();
    let matrix = SignalMatrix::new(defset);

    let planned_eval = matrix.plan_evaluation::<CustomPlanner>(root_targets);

    // phase 0: a, b
    // phase 1: c
    // phase 2: d
    // phase 3: e
    assert_eq!(planned_eval.passes().len(), 4);
    assert!(planned_eval.passes()[0]
      .targets()
      .eq(&[a, b].iter().cloned().collect()));
    assert!(planned_eval.passes()[1]
      .targets()
      .eq(&[c].iter().cloned().collect()));
    assert!(planned_eval.passes()[2]
      .targets()
      .eq(&[d].iter().cloned().collect()));
    assert!(planned_eval.passes()[3]
      .targets()
      .eq(&[e].iter().cloned().collect()));

    let values =
      EvaluationValueMap::new_empty(planned_eval.all_queued_targets());
    let values = planned_eval.run(values);

    assert_eq!(values.get(e).unwrap(), &-9.0);
  }
}
