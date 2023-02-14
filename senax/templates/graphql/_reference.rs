
#[derive(SimpleObject)]
pub struct @{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@@{ rel_name|pascal }@ {
@%- if camel_case %@
@{- def.for_api_response()|fmt_join("
{title}{comment}    pub {var}: {api_type},", "") }@
@%- else %@
@{- def.for_api_response()|fmt_join("
{title}{comment}    #[graphql(name = \"{raw_var}\")]
    pub {var}: {api_type},", "") }@
@%- endif %@
}

impl From<&rel_@{ class_mod }@::_@{ pascal_name }@> for @{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@@{ rel_name|pascal }@ {
    fn from(v: &rel_@{ class_mod }@::_@{ pascal_name }@) -> Self {
        Self {
            @{- def.for_api_response()|fmt_join("
            {var}: v.{var}(){to_api_type},", "") }@
        }
    }
}

impl From<&rel_@{ class_mod }@::_@{ pascal_name }@Cache> for @{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@@{ rel_name|pascal }@ {
    fn from(v: &rel_@{ class_mod }@::_@{ pascal_name }@Cache) -> Self {
        Self {
            @{- def.for_api_response()|fmt_join("
            {var}: v.{var}(){to_api_type},", "") }@
        }
    }
}
@{-"\n"}@