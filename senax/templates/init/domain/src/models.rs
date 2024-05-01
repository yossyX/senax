use ::anyhow::Result;
use ::async_trait::async_trait;
use ::derive_more::Display;
use ::serde::{Deserialize, Serialize};
use ::serde_json::Value;

#[cfg(any(feature = "mock", test))]
macro_rules! get_emu_group {
    ($n:ident, $o:ty, $i:ty) => {
        fn $n(&self) -> Box<$o> {
            Box::new(<$i>::new(self.repo.clone()))
        }
    };
}

#[cfg(any(feature = "mock", test))]
macro_rules! get_emu_repo {
    ($n:ident, $o:ty, $i:ty) => {
        fn $n(&self) -> Box<$o> {
            let mut repo = self._repo.lock().unwrap();
            let repo = repo
                .entry(TypeId::of::<$i>())
                .or_insert_with(|| Box::new(<$i>::default()));
            Box::new(repo.downcast_ref::<$i>().unwrap().clone())
        }
    };
}

// Do not modify this line. (Mod:)

// Do not modify this line. (UseRepo)

pub trait Check_<T: ?Sized> {
    fn check(&self, obj: &T) -> bool;
}

#[cfg_attr(any(feature = "mock", test), mockall::automock)]
#[async_trait]
pub trait Repositories: Send + Sync {
    // Do not modify this line. (Repo)
    async fn begin(&self) -> Result<()>;
    async fn commit(&self) -> Result<()>;
    async fn rollback(&self) -> Result<()>;
}

#[cfg(any(feature = "mock", test))]
#[derive(Clone, Default)]
pub struct EmuRepositories {
    // Do not modify this line. (EmuRepo)
}
#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
impl EmuRepositories {
    pub fn new() -> Self {
        Self::default()
    }
}
#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
#[async_trait]
impl Repositories for EmuRepositories {
    // Do not modify this line. (EmuImpl)
    async fn begin(&self) -> Result<()> {
        // Do not modify this line. (EmuImplStart)
        Ok(())
    }
    async fn commit(&self) -> Result<()> {
        // Do not modify this line. (EmuImplCommit)
        Ok(())
    }
    async fn rollback(&self) -> Result<()> {
        // Do not modify this line. (EmuImplRollback)
        Ok(())
    }
}

pub trait EntityIterator<T: ?Sized + Send + Sync>: Send + Sync {
    fn iter(&self) -> Box<dyn Iterator<Item = &T> + '_>;
    fn into_iter(self) -> Box<dyn Iterator<Item = Box<T>>>;
}

pub trait UpdateIterator<T: ?Sized + Send + Sync>: Send + Sync {
    fn iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut T> + '_>;
}

pub trait MarkForDelete {
    fn mark_for_delete(&mut self);
    fn unmark_for_delete(&mut self);
}

#[derive(Deserialize, Serialize, Display, Copy, Clone, Debug, Default, PartialEq, schemars::JsonSchema)]
#[display(fmt = "{},{}", x, y)]
#[derive(async_graphql::SimpleObject, async_graphql::InputObject)]
#[graphql(input_name = "PointInput")]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl From<&(f64, f64)> for Point {
    fn from(v: &(f64, f64)) -> Self {
        Self { x: v.0, y: v.1 }
    }
}

impl From<(f64, f64)> for Point {
    fn from(v: (f64, f64)) -> Self {
        Self { x: v.0, y: v.1 }
    }
}

impl Point {
    pub fn to_tuple(&self) -> (f64, f64) {
        (self.x, self.y)
    }
}

pub trait ToPoint {
    fn point(&self) -> Point;
}

impl ToPoint for (f64, f64) {
    fn point(&self) -> Point {
        Point::from(self)
    }
}

#[derive(Deserialize, Serialize, Display, Copy, Clone, Debug, Default, PartialEq, schemars::JsonSchema)]
#[display(fmt = "{},{}", lat, lng)]
#[derive(async_graphql::SimpleObject, async_graphql::InputObject)]
#[graphql(input_name = "GeoPointInput")]
pub struct GeoPoint {
    pub lat: f64,
    pub lng: f64,
}

impl From<&(f64, f64)> for GeoPoint {
    fn from(v: &(f64, f64)) -> Self {
        Self { lat: v.0, lng: v.1 }
    }
}

impl From<(f64, f64)> for GeoPoint {
    fn from(v: (f64, f64)) -> Self {
        Self { lat: v.0, lng: v.1 }
    }
}

impl GeoPoint {
    pub fn to_tuple(&self) -> (f64, f64) {
        (self.lat, self.lng)
    }
}

pub trait ToGeoPoint {
    fn geo_point(&self) -> GeoPoint;
}

impl ToGeoPoint for (f64, f64) {
    fn geo_point(&self) -> GeoPoint {
        GeoPoint::from(self)
    }
}

#[derive(
    Deserialize,
    Serialize,
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    schemars::JsonSchema,
    async_graphql::Enum,
)]
pub enum GeometryFilterType {
    Within,
    Intersects,
    Crosses,
    DWithin,
    NotWithin,
    NotIntersects,
    NotCrosses,
    NotDWithin,
}

#[derive(
    Deserialize,
    Serialize,
    Clone,
    Debug,
    PartialEq,
    schemars::JsonSchema,
    async_graphql::InputObject,
)]
#[graphql(input_name = "GeometryFilterInput")]
#[serde(deny_unknown_fields)]
pub struct GeometryFilter {
    pub r#type: GeometryFilterType,
    pub area: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distance: Option<f64>,
}

#[derive(
    Deserialize,
    Serialize,
    Clone,
    Debug,
    Default,
    PartialEq,
    schemars::JsonSchema,
    async_graphql::InputObject,
)]
#[graphql(input_name = "ArrayIntFilterInput")]
#[serde(deny_unknown_fields)]
pub struct ArrayIntFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains: Option<Vec<u64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlaps: Option<Vec<u64>>,
}

#[derive(
    Deserialize,
    Serialize,
    Clone,
    Debug,
    Default,
    PartialEq,
    schemars::JsonSchema,
    async_graphql::InputObject,
)]
#[graphql(input_name = "ArrayStringFilterInput")]
#[serde(deny_unknown_fields)]
pub struct ArrayStringFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlaps: Option<Vec<String>>,
}

#[derive(
    Deserialize,
    Serialize,
    Clone,
    Debug,
    Default,
    PartialEq,
    schemars::JsonSchema,
    async_graphql::InputObject,
)]
#[graphql(input_name = "JsonValueFilterInput")]
#[serde(deny_unknown_fields)]
pub struct JsonValueFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exists: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eq: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#in: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lt: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lte: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gt: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gte: Option<Value>,
}

#[derive(
    Deserialize,
    Serialize,
    Clone,
    Debug,
    Default,
    PartialEq,
    schemars::JsonSchema,
    async_graphql::InputObject,
)]
#[graphql(input_name = "JsonValueWithPathFilterInput")]
#[serde(deny_unknown_fields)]
pub struct JsonValueWithPathFilter {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exists: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eq: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#in: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lt: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lte: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gt: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gte: Option<Value>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Cursor<T> {
    After(T),
    Before(T),
}

#[derive(
    Deserialize,
    Serialize,
    Clone,
    Debug,
    Default,
    PartialEq,
    schemars::JsonSchema,
    async_graphql::InputObject,
)]
#[graphql(input_name = "ImportOption")]
#[serde(deny_unknown_fields)]
pub struct ImportOption {
    pub overwrite: Option<bool>,
    pub ignore: Option<bool>,
}

pub trait Like {
    fn like(&self, c: &str) -> bool;
}
impl Like for str {
    fn like(&self, c: &str) -> bool {
        like::Like::<true>::like(self, c).unwrap_or(false)
    }
}

fn reject_empty<T>(value: &T) -> Result<(), validator::ValidationError>
where
    T: Default + PartialEq,
{
    if value.eq(&Default::default()) {
        Err(validator::ValidationError::new(
            "Empty filters are not allowed.",
        ))
    } else {
        Ok(())
    }
}
@{-"\n"}@