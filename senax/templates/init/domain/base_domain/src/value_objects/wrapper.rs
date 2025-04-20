#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::@{ pascal_name }@;

impl @{ pascal_name }@ {
    pub fn validate(_val: &Self) -> Result<(), validator::ValidationError> {
        Ok(())
    }
}
@{-"\n"}@