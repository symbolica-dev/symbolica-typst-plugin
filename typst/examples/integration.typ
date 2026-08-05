#import "../lib.typ": init

#set page(width: 150mm, height: auto, margin: 18mm)
#set text(size: 10.5pt)

= Rubi integration with genuine steps

#let sym = init(profile: "full")
#let parse = sym.math
#let var = sym.var
#let render = sym.to-typst
#let integrate-with-steps = sym.integrate-with-steps
#let x = var("x")
#let f = parse($x / (x + 1)$)
#let integration = integrate-with-steps(f, x)

#assert(integration.complete)
#assert(integration.steps.any(step => step.depth > 0))

$ integral #render(f) dif x = #render(integration.result) + C $

#for step in integration.steps [
  #h(step.depth * 1.2em)
  #if step.rule == none [*Transformation*] else [*Rule #step.rule*]
  #if step.description != "" [: #step.description]
  #linebreak()
  #h(step.depth * 1.2em)
  $#render(step.input) = #render(step.output)$
  #linebreak()
]
