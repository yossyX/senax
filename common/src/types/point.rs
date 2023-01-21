use anyhow::ensure;
use bytes::{BufMut, BytesMut};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, fmt::Display, str::FromStr};

#[derive(Deserialize, Serialize, Copy, Clone, Debug, Default, PartialEq, JsonSchema)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
async_graphql::scalar!(Point);

impl From<Vec<u8>> for Point {
    fn from(buf: Vec<u8>) -> Self {
        if buf.len() < 21 {
            return Point {
                x: f64::NAN,
                y: f64::NAN,
            };
        }
        let input = buf.as_slice();
        let (bytes, input) = input.split_at(1);
        let endian = u8::from_le_bytes(bytes.try_into().unwrap());
        let (bytes, input) = input.split_at(4);
        let g_type = if endian == 1 {
            u32::from_le_bytes(bytes.try_into().unwrap())
        } else {
            u32::from_be_bytes(bytes.try_into().unwrap())
        };
        if g_type != 1 {
            return Point {
                x: f64::NAN,
                y: f64::NAN,
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
        Point { x, y }
    }
}

impl From<Point> for Vec<u8> {
    fn from(point: Point) -> Self {
        let mut buf = BytesMut::with_capacity(21);
        buf.put_u8(1);
        buf.put_u32_le(1);
        buf.put_f64_le(point.x);
        buf.put_f64_le(point.y);
        buf.to_vec()
    }
}

impl FromStr for Point {
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
        Ok(Point { x, y })
    }
}
impl Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{},{}", self.x, self.y))
    }
}

pub trait ToPoint {
    fn to_point(&self) -> Point;
}

impl ToPoint for Vec<u8> {
    fn to_point(&self) -> Point {
        Point::from(self.clone())
    }
}
