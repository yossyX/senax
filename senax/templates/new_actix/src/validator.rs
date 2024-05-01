
#[allow(dead_code)]
pub fn validate_varchar(value: &str) -> Result<(), validator::ValidationError> {
    for c in value.chars() {
        if c <= '\u{1f}' || ('\u{7f}'..='\u{9f}').contains(&c) {
            return Err(validator::ValidationError::new("non_control_character"));
        }
    }
    Ok(())
}
#[allow(dead_code)]
pub fn validate_array_of_varchar(values: &[String]) -> Result<(), validator::ValidationError> {
    for value in values {
        validate_varchar(value)?;
    }
    Ok(())
}
#[allow(dead_code)]
pub fn validate_text(value: &str) -> Result<(), validator::ValidationError> {
    for c in value.chars() {
        if c <= '\u{1f}' && c != '\u{09}' && c != '\u{0a}' && c != '\u{0d}'
            || ('\u{7f}'..='\u{9f}').contains(&c)
        {
            return Err(validator::ValidationError::new("non_control_character"));
        }
    }
    Ok(())
}
#[allow(dead_code)]
pub(crate) fn validate_unsigned_decimal(
    value: &rust_decimal::Decimal,
) -> Result<(), validator::ValidationError> {
    if value.is_sign_negative() {
        let mut err = validator::ValidationError::new("range");
        err.add_param(::std::borrow::Cow::from("min"), &0.0);
        return Err(err);
    }
    Ok(())
}
