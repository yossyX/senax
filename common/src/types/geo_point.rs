use anyhow::ensure;
use bytes::{BufMut, BytesMut};
use schemars::JsonSchema;
use senax_encoder::{Decode, Encode, Pack, Unpack};
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, fmt::Display, str::FromStr};

#[derive(
    Deserialize,
    Serialize,
    Decode,
    Encode,
    Pack,
    Unpack,
    Copy,
    Clone,
    Debug,
    Default,
    PartialEq,
    JsonSchema,
)]
#[cfg_attr(
    feature = "graphql6",
    derive(graphql6::SimpleObject, graphql6::InputObject)
)]
#[cfg_attr(feature = "graphql6", graphql(input_name = "GeoPointInput"))]
#[cfg_attr(
    feature = "graphql7",
    derive(graphql7::SimpleObject, graphql7::InputObject)
)]
#[cfg_attr(feature = "graphql7", graphql(input_name = "GeoPointInput"))]
#[cfg_attr(feature = "utoipa5", derive(utoipa5::ToSchema))]
#[cfg_attr(feature = "utoipa5", schema(as = GeoPointInput))]
pub struct GeoPoint {
    pub lat: f64,
    pub lng: f64,
}

impl From<&(f64, f64)> for GeoPoint {
    fn from(v: &(f64, f64)) -> Self {
        Self { lat: v.0, lng: v.1 }
    }
}

impl GeoPoint {
    pub fn to_tuple(&self) -> (f64, f64) {
        (self.lat, self.lng)
    }
}
impl From<Vec<u8>> for GeoPoint {
    fn from(input: Vec<u8>) -> Self {
        Self::from(input.as_slice())
    }
}
impl From<&Vec<u8>> for GeoPoint {
    fn from(input: &Vec<u8>) -> Self {
        Self::from(input.as_slice())
    }
}
impl From<&[u8]> for GeoPoint {
    fn from(input: &[u8]) -> Self {
        if input.len() < 21 {
            return GeoPoint {
                lat: f64::NAN,
                lng: f64::NAN,
            };
        }
        let (bytes, input) = input.split_at(1);
        let endian = u8::from_le_bytes(bytes.try_into().unwrap());
        let (bytes, input) = input.split_at(4);
        let g_type = if endian == 1 {
            u32::from_le_bytes(bytes.try_into().unwrap())
        } else {
            u32::from_be_bytes(bytes.try_into().unwrap())
        };
        if g_type != 1 {
            return GeoPoint {
                lat: f64::NAN,
                lng: f64::NAN,
            };
        }
        let (bytes, input) = input.split_at(8);
        let x = if endian == 1 {
            f64::from_le_bytes(bytes.try_into().unwrap())
        } else {
            f64::from_be_bytes(bytes.try_into().unwrap())
        };
        let (bytes, _input) = input.split_at(8);
        let y = if endian == 1 {
            f64::from_le_bytes(bytes.try_into().unwrap())
        } else {
            f64::from_be_bytes(bytes.try_into().unwrap())
        };
        GeoPoint { lat: x, lng: y }
    }
}

impl From<GeoPoint> for Vec<u8> {
    fn from(point: GeoPoint) -> Self {
        let mut buf = BytesMut::with_capacity(21);
        buf.put_u8(1);
        buf.put_u32_le(1);
        buf.put_f64_le(point.lat);
        buf.put_f64_le(point.lng);
        buf.to_vec()
    }
}

impl GeoPoint {
    pub fn to_wkb(&self) -> Vec<u8> {
        let mut buf = BytesMut::with_capacity(21);
        buf.put_u8(1);
        buf.put_u32_le(1);
        buf.put_f64_le(self.lat);
        buf.put_f64_le(self.lng);
        buf.to_vec()
    }
}

impl FromStr for GeoPoint {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let coords: Vec<&str> = s
            .trim_matches(|p| p == '(' || p == ')')
            .split(',')
            .map(|v| v.trim())
            .collect();
        ensure!(coords.len() != 2, "illegal format");
        let x = coords[0].parse::<f64>()?;
        let y = coords[1].parse::<f64>()?;
        Ok(GeoPoint { lat: x, lng: y })
    }
}
impl Display for GeoPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{},{}", self.lat, self.lng))
    }
}

pub trait ToGeoPoint {
    fn to_geo_point(&self) -> GeoPoint;
}

impl ToGeoPoint for Vec<u8> {
    fn to_geo_point(&self) -> GeoPoint {
        GeoPoint::from(self)
    }
}

impl ToGeoPoint for (f64, f64) {
    fn to_geo_point(&self) -> GeoPoint {
        GeoPoint::from(self)
    }
}
