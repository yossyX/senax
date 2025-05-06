@% if non_snake_case -%@
#![allow(non_snake_case)]
@% endif -%@
#[rustfmt::skip]
#[allow(clippy::module_inception)]
#[allow(clippy::overly_complex_bool_expr)]
#[allow(clippy::nonminimal_bool)]
#[allow(clippy::useless_conversion)]
#[allow(clippy::only_used_in_recursion)]
#[allow(clippy::map_identity)]
#[allow(clippy::collapsible_if)]
pub mod models;
pub mod value_objects;
@{-"\n"}@
