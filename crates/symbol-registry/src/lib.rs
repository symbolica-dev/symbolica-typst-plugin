//! Deterministic Symbolica symbol initialization shared by the Wasm engines.

use std::sync::Once;

use symbolica::atom::Atom;
use symbolica_integrate::IntegralFunctions;

static INITIALIZE: Once = Once::new();

/// Register Idenso and Rubi symbols in the same order in every plugin.
pub fn initialize() {
    INITIALIZE.call_once(|| {
        idenso::representations::initialize();

        // Calling any IntegralFunctions constructor forces Rubi's symbol
        // catalog without constructing or retaining the much larger rule set.
        let _ = Atom::Zero.fresnel_s();
    });
}
