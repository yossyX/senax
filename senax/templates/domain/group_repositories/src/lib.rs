#[allow(clippy::module_inception)]
pub mod repositories;

#[cfg(any(feature = "mock", test))]
#[allow(dead_code)]
#[derive(PartialEq)]
enum PartialOrdering_ {
    Less,
    Equal,
    Greater,
    None,
}
#[cfg(any(feature = "mock", test))]
impl From<Option<std::cmp::Ordering>> for PartialOrdering_ {
    fn from(value: Option<std::cmp::Ordering>) -> Self {
        match value {
            Some(std::cmp::Ordering::Less) => PartialOrdering_::Less,
            Some(std::cmp::Ordering::Equal) => PartialOrdering_::Equal,
            Some(std::cmp::Ordering::Greater) => PartialOrdering_::Greater,
            None => PartialOrdering_::None,
        }
    }
}
#[cfg(any(feature = "mock", test))]
impl PartialOrdering_ {
    #[must_use]
    #[allow(dead_code)]
    pub fn then_with<F>(self, f: F) -> PartialOrdering_
    where
        F: FnOnce() -> PartialOrdering_,
    {
        match self {
            PartialOrdering_::Equal => f(),
            _ => self,
        }
    }
}
@{-"\n"}@
