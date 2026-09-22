#import "@preview/symbolica:0.1.0": *

#set page(width: 150mm, height: auto, margin: 16mm)
#set text(size: 10.5pt)

= Symbolica integration with Rubi steps

#let x = parse($x$)
#let integrand = parse($x / (x + 1)$)
#let primitive = integrate(integrand, x)
#let explanation = integrate-with-steps(integrand, x)
#let step-notation = notation(
  calls: (
    "symbolica_integrate::rubi_int": ctx => {
      let (body, variable) = ctx.visual-arguments
      $ integral #body dif #variable $
    },
  ),
)
#let show-step(expression) = to-typst(expression, notation: step-notation)

$ integral #to-typst(integrand) dif x
  = #to-typst(primitive) + C $

#for step in explanation.steps [
  #h(step.depth * 1.2em)
  #if step.rule == none [*Transformation*] else [*Rule #step.rule*]
  #if step.description != "" [: #step.description]
  #linebreak()
  #h(step.depth * 1.2em)
  $#show-step(step.input) = #show-step(step.output)$
  #linebreak()
]
