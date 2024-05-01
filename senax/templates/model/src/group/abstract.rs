@% for (enum_name, column_def) in def.num_enums(false) -%@
pub@{ visibility }@ use super::_base::_@{ mod_name }@::_@{ enum_name|pascal }@;
@% endfor -%@
@% for (enum_name, column_def) in def.str_enums(false) -%@
pub@{ visibility }@ use super::_base::_@{ mod_name }@::_@{ enum_name|pascal }@;
@% endfor -%@
pub@{ visibility }@ use super::_base::_@{ mod_name }@::_@{ pascal_name }@Getter;

@{-"\n"}@