@% if !config.exclude_from_domain -%@
#[allow(clippy::module_inception)]
pub mod impl_domain;
@% endif -%@
#[allow(clippy::module_inception)]
pub mod repositories;
@{-"\n"}@