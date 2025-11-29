// This file includes code from the async-graphql open-source project,
// used under the terms of the MIT OR Apache-2.0 license.

use async_graphql::{InputType, InputValueError, InputValueResult, Value, registry};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Cow;

#[derive(Clone, Debug)]
pub enum MaybeUndefined<T> {
    Undefined,
    Null,
    Value(T),
}

impl<T> MaybeUndefined<T> {
    pub const fn is_undefined(&self) -> bool {
        matches!(self, MaybeUndefined::Undefined)
    }
    pub const fn is_null(&self) -> bool {
        matches!(self, MaybeUndefined::Null)
    }
    pub const fn is_value(&self) -> bool {
        matches!(self, MaybeUndefined::Value(_))
    }
    pub const fn value(&self) -> Option<&T> {
        match self {
            MaybeUndefined::Value(value) => Some(value),
            _ => None,
        }
    }
    pub fn take(self) -> Option<T> {
        match self {
            MaybeUndefined::Value(value) => Some(value),
            _ => None,
        }
    }
}

impl<T: InputType> InputType for MaybeUndefined<T> {
    type RawValueType = T::RawValueType;

    fn type_name() -> Cow<'static, str> {
        T::type_name()
    }

    fn qualified_type_name() -> String {
        T::type_name().to_string()
    }

    fn create_type_info(registry: &mut registry::Registry) -> String {
        T::create_type_info(registry);
        T::type_name().to_string()
    }

    fn parse(value: Option<Value>) -> InputValueResult<Self> {
        match value {
            None => Ok(MaybeUndefined::Undefined),
            Some(Value::Null) => Ok(MaybeUndefined::Null),
            Some(value) => Ok(MaybeUndefined::Value(
                T::parse(Some(value)).map_err(InputValueError::propagate)?,
            )),
        }
    }

    fn to_value(&self) -> Value {
        match self {
            MaybeUndefined::Value(value) => value.to_value(),
            _ => Value::Null,
        }
    }

    fn as_raw_value(&self) -> Option<&Self::RawValueType> {
        if let MaybeUndefined::Value(value) = self {
            value.as_raw_value()
        } else {
            None
        }
    }
}

impl<T: Serialize> Serialize for MaybeUndefined<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            MaybeUndefined::Value(value) => value.serialize(serializer),
            _ => serializer.serialize_none(),
        }
    }
}

impl<'de, T> Deserialize<'de> for MaybeUndefined<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<MaybeUndefined<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|value| match value {
            Some(value) => MaybeUndefined::Value(value),
            None => MaybeUndefined::Null,
        })
    }
}

impl<T> From<MaybeUndefined<T>> for Option<Option<T>> {
    fn from(maybe_undefined: MaybeUndefined<T>) -> Self {
        match maybe_undefined {
            MaybeUndefined::Undefined => None,
            MaybeUndefined::Null => Some(None),
            MaybeUndefined::Value(value) => Some(Some(value)),
        }
    }
}

impl<T> From<Option<Option<T>>> for MaybeUndefined<T> {
    fn from(value: Option<Option<T>>) -> Self {
        match value {
            Some(Some(value)) => Self::Value(value),
            Some(None) => Self::Null,
            None => Self::Undefined,
        }
    }
}

impl<T> validator::ValidateLength<u64> for MaybeUndefined<T>
where
    T: validator::ValidateLength<u64>,
{
    fn length(&self) -> Option<u64> {
        let s = self.value()?;
        T::length(s)
    }
}

impl<'v_a, T, U> validator::ValidateArgs<'v_a> for MaybeUndefined<T>
where
    T: validator::ValidateArgs<'v_a, Args = U>,
{
    type Args = U;

    fn validate_with_args(&self, args: Self::Args) -> Result<(), validator::ValidationErrors> {
        if let Some(nested) = self.value() {
            T::validate_with_args(nested, args)
        } else {
            Ok(())
        }
    }
}

impl<T> validator::ValidateRequired for MaybeUndefined<T> {
    fn is_some(&self) -> bool {
        self.is_value()
    }
}

impl<T> validator::ValidateRegex for MaybeUndefined<T>
where
    T: validator::ValidateRegex,
{
    fn validate_regex(&self, regex: impl validator::AsRegex) -> bool {
        if let Some(h) = self.value() {
            T::validate_regex(h, regex)
        } else {
            true
        }
    }
}

impl<T> validator::ValidateContains for MaybeUndefined<T>
where
    T: validator::ValidateContains,
{
    fn validate_contains(&self, needle: &str) -> bool {
        if let Some(v) = self.value() {
            v.validate_contains(needle)
        } else {
            true
        }
    }
}

macro_rules! impl_val_range {
    ($t:tt) => {
        impl validator::ValidateRange<$t> for MaybeUndefined<$t> {
            fn greater_than(&self, max: $t) -> Option<bool> {
                self.value().map(|r| *r > max)
            }
            fn less_than(&self, min: $t) -> Option<bool> {
                self.value().map(|r| *r < min)
            }
        }
    };
}

impl_val_range!(u8);
impl_val_range!(u16);
impl_val_range!(u32);
impl_val_range!(u64);
impl_val_range!(u128);
impl_val_range!(usize);
impl_val_range!(i8);
impl_val_range!(i16);
impl_val_range!(i32);
impl_val_range!(i64);
impl_val_range!(i128);
impl_val_range!(isize);
impl_val_range!(f32);
impl_val_range!(f64);
