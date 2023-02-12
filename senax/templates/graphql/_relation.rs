
#[derive(SimpleObject)]
pub struct @{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@@{ rel_name|pascal }@ {
@{- def.for_api_response_except(rel_id)|fmt_join("
    pub {var}: {api_type},", "") }@
}

impl From<&rel_@{ class_mod }@::_@{ pascal_name }@> for @{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@@{ rel_name|pascal }@ {
    fn from(v: &rel_@{ class_mod }@::_@{ pascal_name }@) -> Self {
        Self {
            @{- def.for_api_response_except(rel_id)|fmt_join("
            {var}: v.{var}(){to_api_type},", "") }@
        }
    }
}

impl From<rel_@{ class_mod }@::_@{ pascal_name }@Cache> for @{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@@{ rel_name|pascal }@ {
    fn from(v: rel_@{ class_mod }@::_@{ pascal_name }@Cache) -> Self {
        Self {
            @{- def.for_api_response_except(rel_id)|fmt_join("
            {var}: v.{var}(){to_api_type},", "") }@
        }
    }
}

impl From<&rel_@{ class_mod }@::_@{ pascal_name }@Cache> for @{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@@{ rel_name|pascal }@ {
    fn from(v: &rel_@{ class_mod }@::_@{ pascal_name }@Cache) -> Self {
        Self {
            @{- def.for_api_response_except(rel_id)|fmt_join("
            {var}: v.{var}(){to_api_type},", "") }@
        }
    }
}

@%- if def.has_auto_increments() %@

#[derive(Debug, InputObject, Validate, Serialize, Deserialize)]
pub struct Req@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@@{ rel_name|pascal }@ {
@{- def.auto_increments()|fmt_join("
{api_validate}    pub {var}: {api_option_type},", "") }@
@{- def.for_api_request_except(rel_id)|fmt_join("
    pub {var}: {api_type},", "") }@
}

impl From<Req@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@@{ rel_name|pascal }@> for _@{ pascal_name }@Factory {
    fn from(data: Req@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@@{ rel_name|pascal }@) -> Self {
        Self {
@{- def.for_factory()|fmt_join_with_foreign_default("
            {var}: {from_api_rel_type},", "", rel_id) }@
        }
    }
}

@%- endif %@
@{-"\n"}@