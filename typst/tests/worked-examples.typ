// Regression checks kept separate from the user-facing worked examples.
#import "../examples/expression-grid.typ" as expression-grid
#import "../examples/lotka-volterra.typ" as lotka-volterra
#import "../examples/phase-portrait.typ" as phase-portrait
#import "../examples/showcase.typ" as showcase

#assert.eq(
  expression-grid.values.len(),
  expression-grid.x-values.len() * expression-grid.y-values.len(),
)
#assert(calc.abs(expression-grid.values.first().first().re - 91.0) < 1e-10)
#assert(
  calc.abs(expression-grid.values.last().at(2).re - (calc.exp(5) - 1)) < 1e-9,
)
#for row in expression-grid.values {
  for value in row {
    assert(calc.abs(value.im) < 1e-10)
  }
}

#let final = lotka-volterra.trajectory.last()
#assert(calc.abs(final.at(1) - 0.48915) < 0.001)
#assert(calc.abs(final.at(2) - 0.48440) < 0.001)

#assert.eq(
  phase-portrait.values.len(),
  phase-portrait.x-samples * phase-portrait.y-samples,
)

#assert.eq(showcase.solutions.len(), 4)
