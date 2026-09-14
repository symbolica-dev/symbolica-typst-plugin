use symbolica::domains::algebraic::AlgebraicContext;
use symbolica::prelude::*;

fn main() {
    let expression = parse!("2^(1/2)");
    eprintln!("Parsed sqrt(2); constructing its algebraic context...");
    let _ = AlgebraicContext::from_atom(expression.as_view()).expect("sqrt(2) is algebraic");
    eprintln!("OK");
}
