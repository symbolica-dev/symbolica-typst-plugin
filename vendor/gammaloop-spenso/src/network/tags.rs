use symbolica::{
    atom::{
        Atom, AtomCore, AtomOrView, AtomView, FunctionBuilder, NamespacedSymbol, Symbol,
        SymbolAttribute, SymbolBuilder,
    },
    coefficient::CoefficientView,
    printer::{PrintOptions, PrintState, PrintUserData},
    symbol, tag,
};
use symbolica_utils::PrintSettingsExt;

use crate::{
    shadowing::symbolica_utils::SpensoPrintSettings, structure::abstract_index::AIND_SYMBOLS,
};

pub struct SpensoTags {
    pub broadcast: String,
    /// Marks rank-one tensor symbols whose final argument is the tensor slot.
    ///
    /// Symbols carrying this tag must not use representation slots in earlier
    /// arguments; rank-one shorthand rewrites assume that contract.
    pub rank1: String,
    pub rank1_: Symbol,
    pub chain_in: Symbol,
    pub chain_out: Symbol,
    pub chain: Symbol,
    pub trace: Symbol,
    pub upper: String,
    pub lower: String,
    pub bracket: Symbol,
    pub pure_scalar: Symbol,
    pub scalar: Symbol,
    pub tensor: String,
    pub tensor_: Symbol,
    /// Internal wrapper used to restore tensor printing after importing an
    /// Atom whose dynamically registered Rust print callback was not exported.
    pub tensor_display: Symbol,
    pub index: String,
    pub representation: String,
    pub i_: Symbol,
    pub dot: Symbol,
    pub rep_: Symbol,
    pub self_dual: String,
    pub self_dual_: Symbol,
    pub dualizable: String,
    pub dualizable_: Symbol,
}

crate::symbolica_init_lazy_static! {
    pub static SPENSO_TAG, SPENSO_TAG_INNER: SpensoTags = SpensoTags::new;
}

pub fn scalar_store_alias(index: usize) -> Atom {
    FunctionBuilder::new(SPENSO_TAG.scalar)
        .add_arg(Atom::num(
            i64::try_from(index).expect("scalar alias index must fit in i64"),
        ))
        .finish()
}

pub fn scalar_store_alias_index(value: AtomView<'_>) -> Option<usize> {
    let AtomView::Fun(fun) = value else {
        return None;
    };
    if fun.get_symbol() != SPENSO_TAG.scalar || fun.get_nargs() != 1 {
        return None;
    }

    let AtomView::Num(index) = fun.iter().next()? else {
        return None;
    };
    match index.get_coeff_view() {
        CoefficientView::Natural(index, 1, 0, 1) => usize::try_from(index).ok(),
        _ => None,
    }
}

fn typst_builtin_name(name: &str) -> bool {
    matches!(
        name,
        "alpha"
            | "beta"
            | "gamma"
            | "delta"
            | "epsilon"
            | "zeta"
            | "eta"
            | "theta"
            | "iota"
            | "kappa"
            | "lambda"
            | "mu"
            | "nu"
            | "xi"
            | "omicron"
            | "pi"
            | "rho"
            | "sigma"
            | "tau"
            | "upsilon"
            | "phi"
            | "chi"
            | "psi"
            | "omega"
            | "Alpha"
            | "Beta"
            | "Gamma"
            | "Delta"
            | "Epsilon"
            | "Zeta"
            | "Eta"
            | "Theta"
            | "Iota"
            | "Kappa"
            | "Lambda"
            | "Mu"
            | "Nu"
            | "Xi"
            | "Omicron"
            | "Pi"
            | "Rho"
            | "Sigma"
            | "Tau"
            | "Upsilon"
            | "Phi"
            | "Chi"
            | "Psi"
            | "Omega"
    )
}

fn escape_typst_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn typst_tensor_head(symbol: Symbol) -> String {
    let name = symbol.get_stripped_name();
    if name.chars().count() == 1 || typst_builtin_name(name) {
        name.to_owned()
    } else {
        format!(r#"italic("{}")"#, escape_typst_string(name))
    }
}

fn typst_index_source(index: AtomView<'_>, options: &PrintOptions) -> Option<String> {
    if let AtomView::Var(variable) = index {
        let symbol = variable.get_symbol();
        let name = symbol.get_stripped_name();
        if typst_builtin_name(name) {
            return Some(name.to_owned());
        }
    }

    let mut output = String::new();
    index.format(&mut output, options, PrintState::new()).ok()?;
    Some(output)
}

fn tensor_slot(index: AtomView<'_>) -> Option<(AtomView<'_>, bool)> {
    let (slot, lower) = if let AtomView::Fun(dual) = index
        && dual.get_symbol() == AIND_SYMBOLS.dind
        && dual.get_nargs() == 1
    {
        (dual.iter().next()?, true)
    } else {
        (index, false)
    };

    let AtomView::Fun(representation) = slot else {
        return None;
    };
    if !representation
        .get_symbol()
        .has_tag(&SPENSO_TAG.representation)
        || representation.get_nargs() != 2
    {
        return None;
    }

    Some((representation.iter().nth(1)?, lower))
}

/// Print a tagged tensor using native Typst attachments in Spenso's Typst mode.
///
/// Every script occupies the same horizontal column in the top and bottom
/// rows. The opposite row receives a hidden copy, following Physica's tensor
/// layout technique. Plain self-dual slots default to the top row; `dind`
/// explicitly moves a slot to the bottom row.
pub fn tensor_print(
    atom: AtomView<'_>,
    options: &PrintOptions,
    _state: &PrintState,
) -> Option<String> {
    if !options.mode.is_typst() {
        return None;
    }

    let Some(PrintUserData::Integer(encoded)) = options.custom_print_mode.get("spenso") else {
        return None;
    };
    let settings = SpensoPrintSettings::from(*encoded as usize);

    let AtomView::Fun(function) = atom else {
        return None;
    };
    if !function.get_symbol().has_tag(&SPENSO_TAG.tensor) {
        return None;
    }

    let mut top = Vec::new();
    let mut bottom = Vec::new();
    let mut ordinary_arguments = Vec::new();

    for argument in function.iter() {
        let (source, lower) = if let Some((index, lower)) = tensor_slot(argument) {
            (typst_index_source(index, options)?, lower)
        } else if settings.symbol_scripts {
            let mut source = String::new();
            argument
                .format(&mut source, options, PrintState::new())
                .ok()?;
            (source, true)
        } else {
            let mut source = String::new();
            argument
                .format(&mut source, options, PrintState::new())
                .ok()?;
            ordinary_arguments.push(source);
            continue;
        };

        let hidden = format!("std.hide({source})");
        if lower {
            top.push(hidden);
            bottom.push(source);
        } else {
            top.push(source);
            bottom.push(hidden);
        }
    }

    let mut base = typst_tensor_head(function.get_symbol());
    if !ordinary_arguments.is_empty() {
        let separator = if settings.commas { "," } else { " " };
        base.push('(');
        base.push_str(&ordinary_arguments.join(separator));
        base.push(')');
    }

    if top.is_empty() {
        return Some(base);
    }

    Some(format!(
        "attach(#(${base}$,std.hide($zws$)).join(),t:{},b:{})",
        top.join(" "),
        bottom.join(" ")
    ))
}

/// Wrap imported tagged tensors that no longer carry their Rust print callback.
///
/// Symbolica exports tensor tags but intentionally does not export custom Rust
/// functions. This temporary wrapper lets the same generic printer handle such
/// tensors without changing the algebraic Atom.
pub fn prepare_tensor_print(atom: &Atom) -> Atom {
    atom.replace_map(|view, _, output| {
        let AtomView::Fun(function) = view else {
            return;
        };
        let symbol = function.get_symbol();
        if symbol.has_tag(&SPENSO_TAG.tensor) && symbol.get_print_function().is_none() {
            **output = FunctionBuilder::new(SPENSO_TAG.tensor_display)
                .add_arg(view)
                .finish();
        }
    })
}

/// Register a generic tensor or vector head, returning an equivalent existing
/// symbol when it has already been declared.
///
/// Reusing an existing symbol is essential for dynamic frontends: Symbolica
/// deliberately rejects attempts to register the same Rust callback twice.
pub fn register_tensor_symbol(
    name: NamespacedSymbol,
    attributes: Vec<SymbolAttribute>,
    rank_one: bool,
) -> Result<Symbol, String> {
    if let Some(existing) = Symbol::get_symbol(name.clone()) {
        if !existing.has_tag(&SPENSO_TAG.tensor)
            || existing.has_tag(&SPENSO_TAG.rank1) != rank_one
            || existing.get_attributes() != attributes
        {
            return Err(format!(
                "symbol {} already exists with a different tensor declaration",
                existing.get_name()
            ));
        }
        return Ok(existing);
    }

    let tags = if rank_one {
        vec![SPENSO_TAG.tensor.clone(), SPENSO_TAG.rank1.clone()]
    } else {
        vec![SPENSO_TAG.tensor.clone()]
    };
    SymbolBuilder::new(name)
        .with_attributes(attributes)
        .with_tags(tags)
        .build()
        .map_err(|error| error.to_string())
}

/// Builds Symbolica atoms from a symbol and an optional argument list.
///
/// Spenso wildcard heads use the convention that a bare head with no arguments
/// is a variable atom, while a head with arguments is a function atom. This
/// helper keeps that convention in one place.
pub trait SymbolAtomExt {
    fn atom_with_args<'a, A>(self, args: impl IntoIterator<Item = A>) -> Atom
    where
        A: Into<AtomOrView<'a>>;
}

impl SymbolAtomExt for Symbol {
    fn atom_with_args<'a, A>(self, args: impl IntoIterator<Item = A>) -> Atom
    where
        A: Into<AtomOrView<'a>>,
    {
        let mut function = FunctionBuilder::new(self);
        let mut has_args = false;
        for arg in args {
            has_args = true;
            function = function.add_arg(arg);
        }

        if has_args {
            function.finish()
        } else {
            Atom::var(self)
        }
    }
}

macro_rules! define_numbered_tag_family_methods {
    ($(
        $(#[$meta:meta])*
        $vis:vis fn $method:ident => $base_field:ident, $prefix:literal, $symbol_method:ident;
    )*) => {
        $(
            $(#[$meta])*
            $vis fn $method<'a, const N: usize, A: Into<AtomOrView<'a>>>(
                &self,
                args: impl IntoIterator<Item = A>,
            ) -> Atom {
                let symbol = if N == 0 {
                    self.$base_field
                } else {
                    self.$symbol_method(&format!("{}{}_", $prefix, N))
                };
                symbol.atom_with_args(args)
            }
        )*
    };
}

macro_rules! define_numbered_tag_macros {
    ($d:tt; $($macro_name:ident => $method:ident;)*) => {
        $(
            #[macro_export]
            macro_rules! $macro_name {
                ($d n:literal; $d($d arg:expr),* $d(,)?) => {
                    $crate::network::tags::SPENSO_TAG.$method::<$d n, _>(
                        vec![$d($crate::shadowing::IntoAtom::into_atom($d arg)),*],
                    )
                };
                ($d n:literal $d(;)?) => {
                    $crate::network::tags::SPENSO_TAG.$method::<$d n, symbolica::atom::Atom>(
                        std::iter::empty::<symbolica::atom::Atom>(),
                    )
                };
            }
        )*
    };
}

define_numbered_tag_macros!($;
    rank1_ => rank1_;
    tensor_ => tensor_;
    rep_ => rep_;
    self_dual_ => self_dual_;
    dualizable_ => dualizable_;
    dualizable_dual_ => dualizable_dual_;
);

/// Creates a tensor-head symbol tagged with Spenso's generic tensor tag.
///
/// This expands `symbolica::symbol!` at the call site, so the symbol keeps the
/// caller's crate namespace while automatically receiving the Spenso tag. Any
/// Symbolica attributes and settings such as `print = ...` are forwarded, while
/// `tag`/`tags` remain owned by this macro so the tensor tag cannot be skipped.
#[macro_export]
macro_rules! tensor_symbol {
    ($name:ident) => {
        $crate::tensor_symbol!(stringify!($name))
    };
    ($name:ident; $($attr:ident),*) => {
        $crate::tensor_symbol!(stringify!($name); $($attr),*)
    };
    ($name:ident, $($setting:ident = $value:expr),*) => {
        $crate::tensor_symbol!(stringify!($name), $($setting = $value),*)
    };
    ($name:ident; $($attr:ident),+; $($setting:ident = $value:expr),*) => {
        $crate::tensor_symbol!(stringify!($name); $($attr),+; $($setting = $value),*)
    };
    ($id:expr) => {
        $crate::network::tags::register_tensor_symbol(
            symbolica::wrap_symbol!($id),
            Vec::new(),
            false,
        )
        .unwrap_or_else(|error| panic!("{error}"))
    };
    ($id:expr, tag = $tag:expr $(, $($rest:tt)*)?) => {
        compile_error!("tensor_symbol! owns the Spenso tensor tag; do not pass tag = ...")
    };
    ($id:expr, tags = $tags:expr $(, $($rest:tt)*)?) => {
        compile_error!("tensor_symbol! owns the Spenso tensor tag; do not pass tags = ...")
    };
    ($id:expr, $($setting:ident = $value:expr),*) => {
        symbolica::symbol!(
            $id,
            tag = &$crate::network::tags::SPENSO_TAG.tensor,
            $($setting = $value),*
        )
    };
    ($id:expr; $($attr:ident),*) => {
        $crate::network::tags::register_tensor_symbol(
            symbolica::wrap_symbol!($id),
            vec![$(symbolica::atom::SymbolAttribute::$attr),*],
            false,
        )
        .unwrap_or_else(|error| panic!("{error}"))
    };
    ($id:expr; $($attr:ident),+; tag = $tag:expr $(, $($rest:tt)*)?) => {
        compile_error!("tensor_symbol! owns the Spenso tensor tag; do not pass tag = ...")
    };
    ($id:expr; $($attr:ident),+; tags = $tags:expr $(, $($rest:tt)*)?) => {
        compile_error!("tensor_symbol! owns the Spenso tensor tag; do not pass tags = ...")
    };
    ($id:expr; $($attr:ident),+; $($setting:ident = $value:expr),*) => {
        symbolica::symbol!(
            $id;
            $($attr),+;
            tag = &$crate::network::tags::SPENSO_TAG.tensor,
            $($setting = $value),*
        )
    };
}

/// Creates a tensor-head symbol tagged as a Spenso vector.
///
/// This is the rank-one tensor head constructor. It expands
/// `symbolica::symbol!` at the call site, so `vector_symbol!(p)` gets the
/// caller's namespace plus the `tensor` and `rank1` tags.
#[macro_export]
macro_rules! vector_symbol {
    ($name:ident) => {
        $crate::network::tags::register_tensor_symbol(
            symbolica::wrap_symbol!(stringify!($name)),
            Vec::new(),
            true,
        )
        .unwrap_or_else(|error| panic!("{error}"))
    };
    ($name:ident, $($setting:ident = $value:expr),* $(,)?) => {
        symbolica::symbol!(
            stringify!($name),
            tags = [
                &$crate::network::tags::SPENSO_TAG.tensor,
                &$crate::network::tags::SPENSO_TAG.rank1
            ],
            $($setting = $value),*
        )
    };
    ($name:literal) => {
        $crate::network::tags::register_tensor_symbol(
            symbolica::wrap_symbol!($name),
            Vec::new(),
            true,
        )
        .unwrap_or_else(|error| panic!("{error}"))
    };
    ($name:literal, $($setting:ident = $value:expr),* $(,)?) => {
        symbolica::symbol!(
            $name,
            tags = [
                &$crate::network::tags::SPENSO_TAG.tensor,
                &$crate::network::tags::SPENSO_TAG.rank1
            ],
            $($setting = $value),*
        )
    };
}

/// Creates a representation symbol tagged with Spenso's representation tag.
///
/// This expands `symbolica::symbol!` at the call site and adds only the generic
/// representation tag.
#[macro_export]
macro_rules! representation_symbol {
    ($name:ident) => {
        symbolica::symbol!(
            stringify!($name),
            tag = &$crate::network::tags::SPENSO_TAG.representation
        )
    };
    ($name:literal) => {
        symbolica::symbol!(
            $name,
            tag = &$crate::network::tags::SPENSO_TAG.representation
        )
    };
}

/// Creates a self-dual representation symbol.
///
/// This expands `symbolica::symbol!` at the call site and adds the
/// `representation` and `self_dual` tags.
#[macro_export]
macro_rules! self_dual_symbol {
    ($name:ident) => {
        symbolica::symbol!(
            stringify!($name),
            tags = [
                &$crate::network::tags::SPENSO_TAG.representation,
                &$crate::network::tags::SPENSO_TAG.self_dual
            ]
        )
    };
    ($name:literal) => {
        symbolica::symbol!(
            $name,
            tags = [
                &$crate::network::tags::SPENSO_TAG.representation,
                &$crate::network::tags::SPENSO_TAG.self_dual
            ]
        )
    };
}

/// Creates a dualizable representation symbol.
///
/// This expands `symbolica::symbol!` at the call site and adds the
/// `representation` and `dualizable` tags.
#[macro_export]
macro_rules! dualizable_symbol {
    ($name:ident) => {
        symbolica::symbol!(
            stringify!($name),
            tags = [
                &$crate::network::tags::SPENSO_TAG.representation,
                &$crate::network::tags::SPENSO_TAG.dualizable
            ]
        )
    };
    ($name:literal) => {
        symbolica::symbol!(
            $name,
            tags = [
                &$crate::network::tags::SPENSO_TAG.representation,
                &$crate::network::tags::SPENSO_TAG.dualizable
            ]
        )
    };
}

/// Creates an abstract-index symbol tagged with Spenso's index tag.
///
/// This expands `symbolica::symbol!` at the call site and adds the Spenso
/// index tag.
#[macro_export]
macro_rules! index_symbol {
    ($name:ident) => {
        symbolica::symbol!(
            stringify!($name),
            tag = &$crate::network::tags::SPENSO_TAG.index
        )
    };
    ($name:literal) => {
        symbolica::symbol!($name, tag = &$crate::network::tags::SPENSO_TAG.index)
    };
}

/// Creates a function symbol tagged with Spenso's broadcast tag.
///
/// This expands `symbolica::symbol!` at the call site and adds the Spenso
/// broadcast tag.
#[macro_export]
macro_rules! broadcast_symbol {
    ($name:ident) => {
        symbolica::symbol!(
            stringify!($name),
            tag = &$crate::network::tags::SPENSO_TAG.broadcast
        )
    };
    ($name:literal) => {
        symbolica::symbol!($name, tag = &$crate::network::tags::SPENSO_TAG.broadcast)
    };
}

impl SpensoTags {
    fn print_dot(a: AtomView<'_>, opt: &PrintOptions, _state: &PrintState) -> Option<String> {
        match opt.custom_print_mode.get("spenso") {
            Some(PrintUserData::Integer(i)) => {
                let SpensoPrintSettings {
                    parens, with_dim, ..
                } = SpensoPrintSettings::from(*i as usize);

                let AtomView::Fun(f) = a else {
                    return None;
                };

                if f.get_nargs() != 2 {
                    return None;
                }
                let mut argitem = f.iter();
                let a = argitem.next().unwrap();
                let b = argitem.next().unwrap();

                let AtomView::Fun(f_a) = a else {
                    return None;
                };
                let AtomView::Fun(f_b) = b else {
                    return None;
                };

                let a_sym = f_a.get_symbol();
                let b_sym = f_b.get_symbol();

                if a_sym.has_tag(&SPENSO_TAG.rank1) && b_sym.has_tag(&SPENSO_TAG.rank1) {
                    let mut out = String::new();
                    if parens {
                        out.push('(');
                    }
                    f_a.as_view()
                        .format(&mut out, opt, PrintState::new())
                        .unwrap();
                    out.push('.');
                    if with_dim {
                        a.format(&mut out, opt, PrintState::new()).unwrap();
                        out.push('.');
                    }
                    f_b.as_view()
                        .format(&mut out, opt, PrintState::new())
                        .unwrap();
                    if parens {
                        out.push(')');
                    }
                    Some(out)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn new() -> Self {
        let broadcast = tag!("broadcast");
        let upper = tag!("upper");
        let lower = tag!("lower");
        let rank1 = tag!("rank1");
        let tensor = tag!("tensor");
        let index = tag!("index");
        let representation = tag!("representation");
        let self_dual = tag!("self_dual");
        let dualizable = tag!("dualizable");
        Self {
            chain_in: symbol!("in"),
            chain_out: symbol!("out"),
            chain: symbol!(
                "chain";Linear;
                print = |a, opt, _state| {
                    match opt.custom_print_mode.get("spenso") {
                        Some(PrintUserData::Integer(i)) => {
                            let SpensoPrintSettings { parens, .. } =
                                SpensoPrintSettings::from(*i as usize);

                            let AtomView::Fun(f) = a else {
                                return None;
                            };

                            let mut args = f.iter();

                            let in_index = args.next().unwrap();
                            let out_index = args.next().unwrap();

                            let mut s = String::new();
                            in_index.format(&mut s, opt, PrintState::new()).unwrap();
                            if parens {
                                s.push('[');
                            }
                            for a in args {
                                a.format(&mut s, opt, PrintState::new()).unwrap();
                            }
                            if parens {
                                s.push(']');
                            }
                            out_index.format(&mut s, opt, PrintState::new()).unwrap();
                            Some(s)
                        }
                        _ => None,
                    }
                }
            ),
            trace: symbol!(
                "trace";Linear;
                print = |a, opt, _state| {
                    match opt.custom_print_mode.get("spenso") {
                        Some(PrintUserData::Integer(i)) => {
                            let SpensoPrintSettings {
                                parens, with_dim, ..
                            } = SpensoPrintSettings::from(*i as usize);

                            let AtomView::Fun(f) = a else {
                                return None;
                            };

                            let mut args = f.iter();

                            let rep = args.next().unwrap();

                            let mut s = if opt.typst_mode().is_some() {
                                r#"op("Tr")"#
                            } else {
                                "Tr"
                            }
                            .to_string();
                            if with_dim {
                                rep.format(&mut s, opt, PrintState::new()).unwrap();
                            }
                            if parens {
                                s.push('(');
                            }
                            let a = args.next()?;
                            if let AtomView::Fun(f) = a{//} && f.get_symbol() == *CYCLIC {
                                for a in f.iter() {
                                    a.format(&mut s, opt, PrintState::new()).unwrap();
                                }
                            }else{
                                return None;
                            }

                            if parens {
                                s.push(')');
                            }
                            Some(s)
                        }
                        _ => None,
                    }
                }
            ),
            rank1_: symbol!("rank1_", tags = [&tensor, &rank1], print = tensor_print),
            bracket: symbol!("bracket"),
            pure_scalar: symbol!("pure_scalar"),
            scalar: symbol!("scalar"),
            dot: symbol!("dot";Symmetric,Linear; print = Self::print_dot),
            tensor_: symbol!("tensor_", tag = tensor, print = tensor_print),
            tensor_display: symbol!(
                "tensor_display",
                print = |atom, options, state| {
                    let AtomView::Fun(wrapper) = atom else {
                        return None;
                    };
                    if wrapper.get_nargs() != 1 {
                        return None;
                    }
                    tensor_print(wrapper.iter().next()?, options, state)
                }
            ),
            i_: symbol!("i_", tag = &index),
            rep_: symbol!("rep_", tag = &representation),
            self_dual_: symbol!("self_dual_", tags = [&representation, &self_dual]),
            dualizable_: symbol!("dualizable_", tags = [&representation, &dualizable]),
            broadcast,
            upper,
            lower,
            rank1,
            tensor,
            index,
            representation,
            self_dual,
            dualizable,
        }
    }

    define_numbered_tag_family_methods! {
        pub fn rank1_ => rank1_, "rank1", rank_one_tensor_symbol;
        pub fn rep_ => rep_, "rep", representation_symbol;
    }

    pub fn tensor_symbol(&self, name: &str) -> Symbol {
        register_tensor_symbol(symbolica::wrap_symbol!(name), Vec::new(), false)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    pub fn representation_symbol(&self, name: &str) -> Symbol {
        symbol!(name, tag = &self.representation)
    }

    pub fn self_dual_symbol(&self, name: &str) -> Symbol {
        symbol!(name, tags = [&self.representation, &self.self_dual])
    }

    pub fn dualizable_symbol(&self, name: &str) -> Symbol {
        symbol!(name, tags = [&self.representation, &self.dualizable])
    }

    pub fn rank_one_tensor_symbol(&self, name: &str) -> Symbol {
        register_tensor_symbol(symbolica::wrap_symbol!(name), Vec::new(), true)
            .unwrap_or_else(|error| panic!("{error}"))
    }

    define_numbered_tag_family_methods! {
        pub fn tensor_ => tensor_, "tensor", tensor_symbol;
    }

    pub fn chain<'a, 'b, 'c, A, B, F>(
        &self,
        start: A,
        end: B,
        factors: impl IntoIterator<Item = F>,
    ) -> Atom
    where
        A: Into<AtomOrView<'a>>,
        B: Into<AtomOrView<'b>>,
        F: Into<AtomOrView<'c>>,
    {
        let mut f = FunctionBuilder::new(self.chain).add_arg(start).add_arg(end);
        for factor in factors {
            f = f.add_arg(factor);
        }
        f.finish()
    }

    pub fn trace<'a, 'b, R, F>(&self, rep: R, factors: impl IntoIterator<Item = F>) -> Atom
    where
        R: Into<AtomOrView<'a>>,
        F: Into<AtomOrView<'b>>,
    {
        let mut f = FunctionBuilder::new(self.trace).add_arg(rep);
        for factor in factors {
            f = f.add_arg(factor);
        }
        f.finish()
    }

    pub fn reverse_flip_factor(&self, factor: AtomView<'_>) -> Atom {
        let tmp = symbol!("spenso::chain_flip_tmp");
        factor
            .to_owned()
            .replace(self.chain_in)
            .with(tmp)
            .replace(self.chain_out)
            .with(self.chain_in)
            .replace(tmp)
            .with(self.chain_out)
    }

    pub fn reverse_flip_factors(&self, factors: impl IntoIterator<Item = Atom>) -> Vec<Atom> {
        let mut factors = factors.into_iter().collect::<Vec<_>>();
        factors.reverse();
        factors
            .into_iter()
            .map(|factor| self.reverse_flip_factor(factor.as_view()))
            .collect()
    }

    define_numbered_tag_family_methods! {
        pub fn self_dual_ => self_dual_, "self_dual", self_dual_symbol;
        pub fn dualizable_ => dualizable_, "dualizable", dualizable_symbol;
    }

    pub fn dualizable_dual_<'a, const N: usize, A: Into<AtomOrView<'a>>>(
        &self,
        args: impl IntoIterator<Item = A>,
    ) -> Atom {
        AIND_SYMBOLS.dual(self.dualizable_::<N, A>(args))
    }
}

#[cfg(test)]
mod tests {
    use symbolica::{
        atom::{Atom, AtomCore, AtomView, FunctionBuilder, SymbolBuilder},
        function, symbol, wrap_symbol,
    };

    use crate::{cyclic, dind, lor, mink, shadowing::symbolica_utils::SpensoPrintSettings};

    use super::{SPENSO_TAG, SymbolAtomExt, prepare_tensor_print};

    #[test]
    fn numbered_wildcard_macros_build_variables_without_args() {
        let expr = rank1_!(0);
        let AtomView::Var(var) = expr.as_view() else {
            panic!("empty wildcard head should be a variable");
        };

        assert_eq!(var.get_symbol(), SPENSO_TAG.rank1_);
    }

    #[test]
    fn numbered_wildcard_macros_build_functions_with_args() {
        let expr = rank1_!(
            1;
            Atom::var(symbol!("a___")),
            rep_!(2; Atom::var(symbol!("d_")))
        );

        let AtomView::Fun(fun) = expr.as_view() else {
            panic!("wildcard head with args should be a function");
        };

        assert_eq!(
            fun.get_symbol(),
            SPENSO_TAG.rank_one_tensor_symbol("rank11_")
        );
        assert_eq!(fun.get_nargs(), 2);
    }

    #[test]
    fn numbered_representation_families_use_their_own_prefixes() {
        let self_dual = self_dual_!(1; Atom::var(symbol!("d_")));
        let dualizable = dualizable_!(1; Atom::var(symbol!("d_")));

        let AtomView::Fun(self_dual) = self_dual.as_view() else {
            panic!("self-dual wildcard should be a function");
        };
        let AtomView::Fun(dualizable) = dualizable.as_view() else {
            panic!("dualizable wildcard should be a function");
        };

        assert_eq!(
            self_dual.get_symbol(),
            SPENSO_TAG.self_dual_symbol("self_dual1_")
        );
        assert_eq!(
            dualizable.get_symbol(),
            SPENSO_TAG.dualizable_symbol("dualizable1_")
        );
    }

    #[test]
    fn symbol_atom_ext_uses_variable_for_empty_args() {
        let symbol = SPENSO_TAG.tensor_symbol("empty_tensor_pattern");

        assert_eq!(
            symbol.atom_with_args(std::iter::empty::<Atom>()),
            Atom::var(symbol)
        );
        assert!(matches!(
            tensor_!(0; Atom::var(symbol!("a___"))).as_view(),
            AtomView::Fun(_)
        ));
    }

    #[test]
    fn trace_uses_typst_operator_only_in_typst_print_mode() {
        let trace = SPENSO_TAG.trace(Atom::var(symbol!("rep")), [cyclic!(Atom::num(1))]);

        assert_eq!(
            trace
                .printer(SpensoPrintSettings::typst_options())
                .to_string(),
            r#"op("Tr")(1)"#
        );
        assert_eq!(
            trace
                .printer(SpensoPrintSettings::compact().nice_symbolica())
                .to_string(),
            "Tr(1)"
        );
        assert_eq!(
            trace
                .printer(SpensoPrintSettings::typst().nice_symbolica())
                .to_string(),
            "Tr(1)"
        );
    }

    #[test]
    fn tensors_use_physica_style_typst_attachment_columns() {
        let head = crate::tensor_symbol!("spenso_typst_tests::T");
        let tensor = function!(
            head,
            Atom::num(1),
            mink!(4, symbol!("mu")),
            dind!(lor!(4, symbol!("nu")))
        );

        assert_eq!(
            prepare_tensor_print(&tensor)
                .printer(SpensoPrintSettings::typst_options())
                .to_string(),
            "attach(#($T$,std.hide($zws$)).join(),t:std.hide(1) mu std.hide(nu),b:1 std.hide(mu) nu)"
        );
    }

    #[test]
    fn vectors_default_self_dual_slots_to_the_top_row() {
        let head = crate::vector_symbol!("spenso_typst_tests::p");
        let vector = function!(head, mink!(4, symbol!("rho")));

        assert_eq!(
            prepare_tensor_print(&vector)
                .printer(SpensoPrintSettings::typst_options())
                .to_string(),
            "attach(#($p$,std.hide($zws$)).join(),t:rho,b:std.hide(rho))"
        );
    }

    #[test]
    fn tagged_imports_can_recover_the_generic_tensor_printer() {
        let head = SymbolBuilder::new(wrap_symbol!("spenso_typst_tests::R"))
            .with_tags([SPENSO_TAG.tensor.clone()])
            .build()
            .unwrap();
        let tensor = FunctionBuilder::new(head)
            .add_arg(mink!(4, symbol!("sigma")))
            .finish();
        assert!(head.get_print_function().is_none());

        assert_eq!(
            prepare_tensor_print(&tensor)
                .printer(SpensoPrintSettings::typst_options())
                .to_string(),
            "attach(#($R$,std.hide($zws$)).join(),t:sigma,b:std.hide(sigma))"
        );
    }
}
