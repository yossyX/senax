pub fn validate_varchar(value: &str) -> Result<(), validator::ValidationError> {
    for c in value.chars() {
        if c <= '\u{1f}' || ('\u{7f}'..='\u{9f}').contains(&c) {
            return Err(validator::ValidationError::new("non_control_character"));
        }
    }
    Ok(())
}

pub fn validate_varchar_opt(
    value: &crate::MaybeUndefined<std::string::String>,
) -> Result<(), validator::ValidationError> {
    let Some(value) = value.value() else {
        return Ok(());
    };
    for c in value.chars() {
        if c <= '\u{1f}' || ('\u{7f}'..='\u{9f}').contains(&c) {
            return Err(validator::ValidationError::new("non_control_character"));
        }
    }
    Ok(())
}

pub fn validate_array_of_varchar(values: &[String]) -> Result<(), validator::ValidationError> {
    for value in values {
        validate_varchar(value)?;
    }
    Ok(())
}

pub fn validate_array_of_varchar_opt(
    values: &crate::MaybeUndefined<Vec<String>>,
) -> Result<(), validator::ValidationError> {
    let Some(values) = values.value() else {
        return Ok(());
    };
    for value in values {
        validate_varchar(value)?;
    }
    Ok(())
}

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

pub fn validate_text_opt(
    value: &crate::MaybeUndefined<std::string::String>,
) -> Result<(), validator::ValidationError> {
    let Some(value) = value.value() else {
        return Ok(());
    };
    for c in value.chars() {
        if c <= '\u{1f}' && c != '\u{09}' && c != '\u{0a}' && c != '\u{0d}'
            || ('\u{7f}'..='\u{9f}').contains(&c)
        {
            return Err(validator::ValidationError::new("non_control_character"));
        }
    }
    Ok(())
}

pub fn validate_unsigned_decimal(
    value: &rust_decimal::Decimal,
) -> Result<(), validator::ValidationError> {
    if value.is_sign_negative() {
        let mut err = validator::ValidationError::new("range");
        err.add_param(::std::borrow::Cow::from("min"), &0.0);
        return Err(err);
    }
    Ok(())
}

pub fn validate_unsigned_decimal_opt(
    value: &crate::MaybeUndefined<rust_decimal::Decimal>,
) -> Result<(), validator::ValidationError> {
    let Some(value) = value.value() else {
        return Ok(());
    };
    if value.is_sign_negative() {
        let mut err = validator::ValidationError::new("range");
        err.add_param(::std::borrow::Cow::from("min"), &0.0);
        return Err(err);
    }
    Ok(())
}

pub fn validate_json_object(value: &serde_json::Value) -> Result<(), validator::ValidationError> {
    if !value.is_object() && !value.is_array() && !value.is_null() {
        return Err(validator::ValidationError::new("object"));
    }
    Ok(())
}

pub fn validate_json_object_opt(
    value: &crate::MaybeUndefined<serde_json::Value>,
) -> Result<(), validator::ValidationError> {
    let Some(value) = value.value() else {
        return Ok(());
    };
    if !value.is_object() && !value.is_array() && !value.is_null() {
        return Err(validator::ValidationError::new("object"));
    }
    Ok(())
}
@{-"\n"}@