use nom::branch::alt;
use nom::character::complete::{digit1, line_ending, multispace0, multispace1};
use nom::character::is_alphanumeric;
use nom::combinator::{map, not, peek};
use nom::{IResult, InputLength};
use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use std::str;
use std::str::FromStr;

use super::column::Column;
use super::keywords::{escape, sql_keyword};
use nom::bytes::complete::{is_not, tag, tag_no_case, take, take_while1};
use nom::combinator::opt;
use nom::error::{ErrorKind, ParseError};
use nom::multi::{fold_many0, many0};
use nom::sequence::{delimited, pair, preceded, terminated, tuple};

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SqlType {
    Bool,
    Char(u32),
    Varchar(u32),
    Int,
    UnsignedInt,
    Smallint,
    UnsignedSmallint,
    Bigint,
    UnsignedBigint,
    Tinyint,
    UnsignedTinyint,
    Blob,
    Longblob,
    Mediumblob,
    Tinyblob,
    Double,
    Float,
    Real,
    Tinytext,
    Mediumtext,
    Longtext,
    Text,
    Date,
    Time,
    DateTime(u16),
    Timestamp(u16),
    Binary(u16),
    Varbinary(u16),
    Enum(Vec<Literal>),
    Set(Vec<Literal>),
    Decimal(u16, u16),
    Json,
    Point,
    Geometry,
}

impl fmt::Display for SqlType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            SqlType::Bool => write!(f, "BOOL"),
            SqlType::Char(len) => write!(f, "CHAR({})", len),
            SqlType::Varchar(len) => write!(f, "VARCHAR({})", len),
            SqlType::Int => write!(f, "INT"),
            SqlType::UnsignedInt => write!(f, "INT UNSIGNED"),
            SqlType::Smallint => write!(f, "SMALLINT"),
            SqlType::UnsignedSmallint => write!(f, "SMALLINT UNSIGNED"),
            SqlType::Bigint => write!(f, "BIGINT"),
            SqlType::UnsignedBigint => write!(f, "BIGINT UNSIGNED"),
            SqlType::Tinyint => write!(f, "TINYINT"),
            SqlType::UnsignedTinyint => write!(f, "TINYINT UNSIGNED"),
            SqlType::Blob => write!(f, "BLOB"),
            SqlType::Longblob => write!(f, "LONGBLOB"),
            SqlType::Mediumblob => write!(f, "MEDIUMBLOB"),
            SqlType::Tinyblob => write!(f, "TINYBLOB"),
            SqlType::Double => write!(f, "DOUBLE"),
            SqlType::Float => write!(f, "FLOAT"),
            SqlType::Real => write!(f, "REAL"),
            SqlType::Tinytext => write!(f, "TINYTEXT"),
            SqlType::Mediumtext => write!(f, "MEDIUMTEXT"),
            SqlType::Longtext => write!(f, "LONGTEXT"),
            SqlType::Text => write!(f, "TEXT"),
            SqlType::Date => write!(f, "DATE"),
            SqlType::Time => write!(f, "TIME"),
            SqlType::DateTime(len) => {
                if len > 0 {
                    write!(f, "DATETIME({})", len)
                } else {
                    write!(f, "DATETIME")
                }
            }
            SqlType::Timestamp(len) => {
                if len > 0 {
                    write!(f, "TIMESTAMP({})", len)
                } else {
                    write!(f, "TIMESTAMP")
                }
            }
            SqlType::Binary(len) => write!(f, "BINARY({})", len),
            SqlType::Varbinary(len) => write!(f, "VARBINARY({})", len),
            SqlType::Enum(ref v) => write!(
                f,
                "ENUM({})",
                v.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            SqlType::Set(ref v) => write!(
                f,
                "SET({})",
                v.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            SqlType::Decimal(m, d) => write!(f, "DECIMAL({}, {})", m, d),
            SqlType::Json => write!(f, "JSON"),
            SqlType::Point => write!(f, "POINT"),
            SqlType::Geometry => write!(f, "GEOMETRY"),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Real {
    pub integral: i32,
    pub fractional: i32,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemPlaceholder {
    QuestionMark,
    DollarNumber(i32),
    ColonNumber(i32),
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ItemPlaceholder {
    fn to_string(&self) -> String {
        match *self {
            ItemPlaceholder::QuestionMark => "?".to_string(),
            ItemPlaceholder::DollarNumber(ref i) => format!("${}", i),
            ItemPlaceholder::ColonNumber(ref i) => format!(":{}", i),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Literal {
    Null,
    Integer(i64),
    UnsignedInteger(u64),
    FixedPoint(Real),
    String(String),
    Blob(Vec<u8>),
    CurrentTime,
    CurrentDate,
    CurrentTimestamp,
    Placeholder(ItemPlaceholder),
}

impl From<i64> for Literal {
    fn from(i: i64) -> Self {
        Literal::Integer(i)
    }
}

impl From<u64> for Literal {
    fn from(i: u64) -> Self {
        Literal::UnsignedInteger(i)
    }
}

impl From<i32> for Literal {
    fn from(i: i32) -> Self {
        Literal::Integer(i.into())
    }
}

impl From<u32> for Literal {
    fn from(i: u32) -> Self {
        Literal::UnsignedInteger(i.into())
    }
}

impl From<String> for Literal {
    fn from(s: String) -> Self {
        Literal::String(s)
    }
}

impl<'a> From<&'a str> for Literal {
    fn from(s: &'a str) -> Self {
        Literal::String(String::from(s))
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for Literal {
    fn to_string(&self) -> String {
        match *self {
            Literal::Null => "NULL".to_string(),
            Literal::Integer(ref i) => format!("{}", i),
            Literal::UnsignedInteger(ref i) => format!("{}", i),
            Literal::FixedPoint(ref f) => format!("{}.{}", f.integral, f.fractional),
            Literal::String(ref s) => format!("'{}'", s.replace('\'', "''")),
            Literal::Blob(ref bv) => bv
                .iter()
                .map(|v| format!("{:x}", v))
                .collect::<Vec<String>>()
                .join(" "),
            Literal::CurrentTime => "CURRENT_TIME".to_string(),
            Literal::CurrentDate => "CURRENT_DATE".to_string(),
            Literal::CurrentTimestamp => "CURRENT_TIMESTAMP".to_string(),
            Literal::Placeholder(ref item) => item.to_string(),
        }
    }
}

impl Literal {
    pub fn to_raw_string(&self) -> String {
        match *self {
            Literal::Integer(ref i) => format!("{}", i),
            Literal::UnsignedInteger(ref i) => format!("{}", i),
            Literal::FixedPoint(ref f) => format!("{}.{}", f.integral, f.fractional),
            Literal::String(ref s) => s.clone(),
            _ => "".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableKey {
    PrimaryKey(Vec<Column>),
    UniqueKey(String, Vec<Column>),
    FulltextKey(String, Vec<Column>, Option<String>),
    Key(String, Vec<Column>),
    SpatialKey(String, Vec<Column>),
    Constraint(
        String,
        Vec<Column>,
        String,
        Vec<Column>,
        Option<ReferenceOption>,
        Option<ReferenceOption>,
    ),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, derive_more::Display)]
pub enum ReferenceOption {
    #[display(fmt = "RESTRICT")]
    Restrict,
    #[display(fmt = "CASCADE")]
    Cascade,
    #[display(fmt = "SET NULL")]
    SetNull,
    #[display(fmt = "NO ACTION")]
    NoAction,
    #[display(fmt = "SET DEFAULT")]
    SetDefault,
}

pub fn reference_option(i: &[u8]) -> IResult<&[u8], ReferenceOption> {
    alt((
        map(tag_no_case("RESTRICT"), |_| ReferenceOption::Restrict),
        map(tag_no_case("CASCADE"), |_| ReferenceOption::Cascade),
        map(tag_no_case("SET NULL"), |_| ReferenceOption::SetNull),
        map(tag_no_case("NO ACTION"), |_| ReferenceOption::NoAction),
        map(tag_no_case("SET DEFAULT"), |_| ReferenceOption::SetDefault),
    ))(i)
}

impl fmt::Display for TableKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TableKey::PrimaryKey(ref columns) => {
                write!(f, "PRIMARY KEY ")?;
                write!(
                    f,
                    "({})",
                    columns
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            TableKey::UniqueKey(ref name, ref columns) => {
                write!(f, "UNIQUE KEY {} ", escape(name))?;
                write!(
                    f,
                    "({})",
                    columns
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            TableKey::FulltextKey(ref name, ref columns, ref parser) => {
                write!(f, "FULLTEXT KEY {} ", escape(name))?;
                write!(
                    f,
                    "({})",
                    columns
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
                if let Some(parser) = parser {
                    write!(f, "/*!50100 WITH PARSER `{}` */", parser)?;
                }
                Ok(())
            }
            TableKey::Key(ref name, ref columns) => {
                write!(f, "KEY {} ", escape(name))?;
                write!(
                    f,
                    "({})",
                    columns
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            TableKey::SpatialKey(ref name, ref columns) => {
                write!(f, "SPATIAL KEY {} ", escape(name))?;
                write!(
                    f,
                    "({})",
                    columns
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            TableKey::Constraint(
                ref name,
                ref columns,
                ref table,
                ref foreign,
                ref on_delete,
                ref on_update,
            ) => {
                write!(f, "CONSTRAINT {} FOREIGN KEY ", escape(name))?;
                write!(
                    f,
                    "({})",
                    columns
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
                write!(f, " REFERENCES {} ", escape(table))?;
                write!(
                    f,
                    "({})",
                    foreign
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
                if let Some(on_delete) = on_delete {
                    write!(f, " ON DELETE {}", &on_delete.to_string())?;
                }
                if let Some(on_update) = on_update {
                    write!(f, " ON UPDATE {}", &on_update.to_string())?;
                }
                Ok(())
            }
        }
    }
}

#[inline]
pub fn is_sql_identifier(chr: u8) -> bool {
    is_alphanumeric(chr) || chr == b'_' || chr == b'@'
}

#[inline]
pub fn is_quoted_sql_identifier(chr: u8) -> bool {
    chr > b' '
        && chr != b'`'
        && chr != b'['
        && chr != b']'
        && chr != b','
        && chr != b'('
        && chr != b')'
        && chr != 0x7f
}

#[inline]
fn len_as_u16(len: &[u8]) -> u16 {
    match str::from_utf8(len) {
        Ok(s) => match u16::from_str(s) {
            Ok(v) => v,
            Err(e) => std::panic::panic_any(e),
        },
        Err(e) => std::panic::panic_any(e),
    }
}

pub fn len_as_u32(len: &[u8]) -> u32 {
    match str::from_utf8(len) {
        Ok(s) => match u32::from_str(s) {
            Ok(v) => v,
            Err(e) => std::panic::panic_any(e),
        },
        Err(e) => std::panic::panic_any(e),
    }
}

fn precision_helper(i: &[u8]) -> IResult<&[u8], (u16, Option<u16>)> {
    let (remaining_input, (m, d)) = tuple((
        digit1,
        opt(preceded(tag(","), preceded(multispace0, digit1))),
    ))(i)?;

    Ok((remaining_input, (len_as_u16(m), d.map(len_as_u16))))
}

pub fn precision(i: &[u8]) -> IResult<&[u8], (u16, Option<u16>)> {
    delimited(tag("("), precision_helper, tag(")"))(i)
}

fn opt_signed(i: &[u8]) -> IResult<&[u8], Option<&[u8]>> {
    opt(alt((tag_no_case("unsigned"), tag_no_case("signed"))))(i)
}

fn opt_unsigned(i: &[u8]) -> IResult<&[u8], Option<&[u8]>> {
    opt(tag_no_case("unsigned"))(i)
}

fn delim_digit(i: &[u8]) -> IResult<&[u8], &[u8]> {
    delimited(tag("("), digit1, tag(")"))(i)
}

// TODO: rather than copy paste these functions, should create a function that returns a parser
// based on the sql int type, just like nom does
fn tiny_int(i: &[u8]) -> IResult<&[u8], SqlType> {
    let (remaining_input, (_, _len, _, signed)) = tuple((
        tag_no_case("tinyint"),
        opt(delim_digit),
        multispace0,
        opt_signed,
    ))(i)?;

    match signed {
        Some(sign) => {
            if str::from_utf8(sign)
                .unwrap()
                .eq_ignore_ascii_case("unsigned")
            {
                Ok((remaining_input, SqlType::UnsignedTinyint))
            } else {
                Ok((remaining_input, SqlType::Tinyint))
            }
        }
        None => Ok((remaining_input, SqlType::Tinyint)),
    }
}

// TODO: rather than copy paste these functions, should create a function that returns a parser
// based on the sql int type, just like nom does
fn big_int(i: &[u8]) -> IResult<&[u8], SqlType> {
    let (remaining_input, (_, _len, _, signed)) = tuple((
        tag_no_case("bigint"),
        opt(delim_digit),
        multispace0,
        opt_signed,
    ))(i)?;

    match signed {
        Some(sign) => {
            if str::from_utf8(sign)
                .unwrap()
                .eq_ignore_ascii_case("unsigned")
            {
                Ok((remaining_input, SqlType::UnsignedBigint))
            } else {
                Ok((remaining_input, SqlType::Bigint))
            }
        }
        None => Ok((remaining_input, SqlType::Bigint)),
    }
}

// TODO: rather than copy paste these functions, should create a function that returns a parser
// based on the sql int type, just like nom does
fn sql_int_type(i: &[u8]) -> IResult<&[u8], SqlType> {
    let (remaining_input, (_, _len, _, signed)) = tuple((
        alt((tag_no_case("integer"), tag_no_case("int"))),
        opt(delim_digit),
        multispace0,
        opt_signed,
    ))(i)?;

    match signed {
        Some(sign) => {
            if str::from_utf8(sign)
                .unwrap()
                .eq_ignore_ascii_case("unsigned")
            {
                Ok((remaining_input, SqlType::UnsignedInt))
            } else {
                Ok((remaining_input, SqlType::Int))
            }
        }
        None => Ok((remaining_input, SqlType::Int)),
    }
}
fn small_int_type(i: &[u8]) -> IResult<&[u8], SqlType> {
    let (remaining_input, (_, _len, _, signed)) = tuple((
        tag_no_case("smallint"),
        opt(delim_digit),
        multispace0,
        opt_signed,
    ))(i)?;

    match signed {
        Some(sign) => {
            if str::from_utf8(sign)
                .unwrap()
                .eq_ignore_ascii_case("unsigned")
            {
                Ok((remaining_input, SqlType::UnsignedSmallint))
            } else {
                Ok((remaining_input, SqlType::Smallint))
            }
        }
        None => Ok((remaining_input, SqlType::Smallint)),
    }
}

// TODO(malte): not strictly ok to treat DECIMAL and NUMERIC as identical; the
// former has "at least" M precision, the latter "exactly".
// See https://dev.mysql.com/doc/refman/5.7/en/precision-math-decimal-characteristics.html
fn decimal_or_numeric(i: &[u8]) -> IResult<&[u8], SqlType> {
    let (remaining_input, (_, precision, _, _unsigned)) = tuple((
        alt((tag_no_case("decimal"), tag_no_case("numeric"))),
        opt(precision),
        multispace0,
        opt_unsigned,
    ))(i)?;

    match precision {
        None => Ok((remaining_input, SqlType::Decimal(32, 0))),
        Some((m, None)) => Ok((remaining_input, SqlType::Decimal(m, 0))),
        Some((m, Some(d))) => Ok((remaining_input, SqlType::Decimal(m, d))),
    }
}

fn type_identifier_first_half(i: &[u8]) -> IResult<&[u8], SqlType> {
    alt((
        tiny_int,
        big_int,
        sql_int_type,
        small_int_type,
        map(tag_no_case("bool"), |_| SqlType::Bool),
        map(
            tuple((
                tag_no_case("char"),
                delim_digit,
                multispace0,
                opt(tag_no_case("binary")),
            )),
            |t| SqlType::Char(len_as_u32(t.1)),
        ),
        map(preceded(tag_no_case("datetime"), opt(delim_digit)), |fsp| {
            SqlType::DateTime(match fsp {
                Some(fsp) => len_as_u16(fsp),
                None => 0_u16,
            })
        }),
        map(tag_no_case("date"), |_| SqlType::Date),
        map(
            preceded(tag_no_case("timestamp"), opt(delim_digit)),
            |fsp| {
                SqlType::Timestamp(match fsp {
                    Some(fsp) => len_as_u16(fsp),
                    None => 0_u16,
                })
            },
        ),
        map(tag_no_case("time"), |_| SqlType::Time),
        map(
            tuple((tag_no_case("double"), multispace0, opt_unsigned)),
            |_| SqlType::Double,
        ),
        map(
            terminated(
                preceded(
                    tag_no_case("enum"),
                    delimited(tag("("), value_list, tag(")")),
                ),
                multispace0,
            ),
            SqlType::Enum,
        ),
        map(
            terminated(
                preceded(
                    tag_no_case("set"),
                    delimited(tag("("), value_list, tag(")")),
                ),
                multispace0,
            ),
            SqlType::Set,
        ),
        map(
            tuple((
                tag_no_case("float"),
                multispace0,
                opt(precision),
                multispace0,
                opt_unsigned,
            )),
            |_| SqlType::Float,
        ),
        map(
            tuple((tag_no_case("real"), multispace0, opt_unsigned)),
            |_| SqlType::Real,
        ),
        map(tag_no_case("text"), |_| SqlType::Text),
        map(
            tuple((
                tag_no_case("varchar"),
                delim_digit,
                multispace0,
                opt(tag_no_case("binary")),
            )),
            |t| SqlType::Varchar(len_as_u32(t.1)),
        ),
        map(tag_no_case("json"), |_| SqlType::Json),
        map(tag_no_case("point"), |_| SqlType::Point),
        map(tag_no_case("geometry"), |_| SqlType::Geometry),
        decimal_or_numeric,
    ))(i)
}

fn type_identifier_second_half(i: &[u8]) -> IResult<&[u8], SqlType> {
    alt((
        map(
            tuple((tag_no_case("binary"), delim_digit, multispace0)),
            |t| SqlType::Binary(len_as_u16(t.1)),
        ),
        map(tag_no_case("blob"), |_| SqlType::Blob),
        map(tag_no_case("longblob"), |_| SqlType::Longblob),
        map(tag_no_case("mediumblob"), |_| SqlType::Mediumblob),
        map(tag_no_case("mediumtext"), |_| SqlType::Mediumtext),
        map(tag_no_case("longtext"), |_| SqlType::Longtext),
        map(tag_no_case("tinyblob"), |_| SqlType::Tinyblob),
        map(tag_no_case("tinytext"), |_| SqlType::Tinytext),
        map(
            tuple((tag_no_case("varbinary"), delim_digit, multispace0)),
            |t| SqlType::Varbinary(len_as_u16(t.1)),
        ),
    ))(i)
}

// A SQL type specifier.
pub fn type_identifier(i: &[u8]) -> IResult<&[u8], SqlType> {
    alt((type_identifier_first_half, type_identifier_second_half))(i)
}

// Parses a SQL column identifier in the table.column format
pub fn column_identifier_no_alias(i: &[u8]) -> IResult<&[u8], Column> {
    let (remaining_input, (column, len)) =
        tuple((sql_identifier, opt(delimited(tag("("), digit1, tag(")")))))(i)?;
    Ok((
        remaining_input,
        Column {
            name: str::from_utf8(column).unwrap().to_string(),
            query: None,
            len: len.map(|l| u32::from_str(str::from_utf8(l).unwrap()).unwrap()),
            desc: false,
        },
    ))
}
pub fn column_identifier_query(i: &[u8]) -> IResult<&[u8], Column> {
    let (remaining_input, query) =
        delimited(tag("("), take_until_unbalanced('(', ')'), tag(")"))(i)?;
    Ok((
        remaining_input,
        Column {
            name: "".to_string(),
            query: Some(str::from_utf8(query).unwrap().to_string()),
            len: None,
            desc: false,
        },
    ))
}

// Parses a SQL identifier (alphanumeric1 and "_").
pub fn sql_identifier(i: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((
        preceded(not(peek(sql_keyword)), take_while1(is_sql_identifier)),
        delimited(tag("`"), take_while1(is_quoted_sql_identifier), tag("`")),
        delimited(tag("["), take_while1(is_quoted_sql_identifier), tag("]")),
    ))(i)
}
pub fn take_until_unbalanced(
    opening_bracket: char,
    closing_bracket: char,
) -> impl Fn(&[u8]) -> IResult<&[u8], &[u8]> {
    move |i: &[u8]| {
        let mut index = 0;
        let mut bracket_counter = 0;
        while index < i.len() {
            match i[index] {
                b'\\' => {
                    index += 1;
                }
                c if c == opening_bracket as u8 => {
                    bracket_counter += 1;
                }
                c if c == closing_bracket as u8 => {
                    bracket_counter -= 1;
                }
                _ => {}
            };
            if bracket_counter == -1 {
                return Ok((&i[index..], &i[0..index]));
            };
            index += 1;
        }

        if bracket_counter == 0 {
            Ok(("".as_bytes(), i))
        } else {
            Err(nom::Err::Error(nom::error::Error::from_error_kind(
                i,
                ErrorKind::TakeUntil,
            )))
        }
    }
}

pub(crate) fn eof<I: Copy + InputLength, E: ParseError<I>>(input: I) -> IResult<I, I, E> {
    if input.input_len() == 0 {
        Ok((input, input))
    } else {
        Err(nom::Err::Error(E::from_error_kind(input, ErrorKind::Eof)))
    }
}

// Parse a terminator that ends a SQL statement.
pub fn statement_terminator(i: &[u8]) -> IResult<&[u8], ()> {
    let (remaining_input, _) =
        delimited(multispace0, alt((tag(";"), line_ending, eof)), multispace0)(i)?;

    Ok((remaining_input, ()))
}

pub(crate) fn ws_sep_comma(i: &[u8]) -> IResult<&[u8], &[u8]> {
    delimited(multispace0, tag(","), multispace0)(i)
}

pub(crate) fn ws_sep_equals<'a, I>(i: I) -> IResult<I, I>
where
    I: nom::InputTakeAtPosition + nom::InputTake + nom::Compare<&'a str>,
    // Compare required by tag
    <I as nom::InputTakeAtPosition>::Item: nom::AsChar + Clone,
    // AsChar and Clone required by multispace0
{
    delimited(multispace0, tag("="), multispace0)(i)
}

// Integer literal value
pub fn integer_literal(i: &[u8]) -> IResult<&[u8], Literal> {
    map(pair(opt(tag("-")), digit1), |tup| {
        let mut intval = i64::from_str(str::from_utf8(tup.1).unwrap()).unwrap();
        if (tup.0).is_some() {
            intval *= -1;
        }
        Literal::Integer(intval)
    })(i)
}

fn unpack(v: &[u8]) -> i32 {
    i32::from_str(str::from_utf8(v).unwrap()).unwrap()
}

// Floating point literal value
pub fn float_literal(i: &[u8]) -> IResult<&[u8], Literal> {
    map(tuple((opt(tag("-")), digit1, tag("."), digit1)), |tup| {
        Literal::FixedPoint(Real {
            integral: if (tup.0).is_some() {
                -unpack(tup.1)
            } else {
                unpack(tup.1)
            },
            fractional: unpack(tup.3),
        })
    })(i)
}

/// String literal value
fn raw_string_quoted(input: &[u8], is_single_quote: bool) -> IResult<&[u8], Vec<u8>> {
    // TODO: clean up these assignments. lifetimes and temporary values made it difficult
    let quote_slice: &[u8] = if is_single_quote { b"\'" } else { b"\"" };
    let double_quote_slice: &[u8] = if is_single_quote { b"\'\'" } else { b"\"\"" };
    let backslash_quote: &[u8] = if is_single_quote { b"\\\'" } else { b"\\\"" };
    delimited(
        tag(quote_slice),
        fold_many0(
            alt((
                is_not(backslash_quote),
                map(tag(double_quote_slice), |_| -> &[u8] {
                    if is_single_quote {
                        b"\'"
                    } else {
                        b"\""
                    }
                }),
                map(tag("\\\\"), |_| &b"\\"[..]),
                map(tag("\\b"), |_| &b"\x7f"[..]),
                map(tag("\\r"), |_| &b"\r"[..]),
                map(tag("\\n"), |_| &b"\n"[..]),
                map(tag("\\t"), |_| &b"\t"[..]),
                map(tag("\\0"), |_| &b"\0"[..]),
                map(tag("\\Z"), |_| &b"\x1A"[..]),
                preceded(tag("\\"), take(1usize)),
            )),
            Vec::new,
            |mut acc: Vec<u8>, bytes: &[u8]| {
                acc.extend(bytes);
                acc
            },
        ),
        tag(quote_slice),
    )(input)
}

fn raw_string_single_quoted(i: &[u8]) -> IResult<&[u8], Vec<u8>> {
    raw_string_quoted(i, true)
}

fn raw_string_double_quoted(i: &[u8]) -> IResult<&[u8], Vec<u8>> {
    raw_string_quoted(i, false)
}

pub fn string_literal(i: &[u8]) -> IResult<&[u8], Literal> {
    map(
        alt((raw_string_single_quoted, raw_string_double_quoted)),
        |bytes| match String::from_utf8(bytes) {
            Ok(s) => Literal::String(s),
            Err(err) => Literal::Blob(err.into_bytes()),
        },
    )(i)
}

// Any literal value.
pub fn literal(i: &[u8]) -> IResult<&[u8], Literal> {
    alt((
        float_literal,
        integer_literal,
        string_literal,
        map(tag_no_case("null"), |_| Literal::Null),
        map(tag_no_case("current_timestamp"), |_| {
            Literal::CurrentTimestamp
        }),
        map(tag_no_case("current_date"), |_| Literal::CurrentDate),
        map(tag_no_case("current_time"), |_| Literal::CurrentTime),
        map(tag("?"), |_| {
            Literal::Placeholder(ItemPlaceholder::QuestionMark)
        }),
        map(preceded(tag(":"), digit1), |num| {
            let value = i32::from_str(str::from_utf8(num).unwrap()).unwrap();
            Literal::Placeholder(ItemPlaceholder::ColonNumber(value))
        }),
        map(preceded(tag("$"), digit1), |num| {
            let value = i32::from_str(str::from_utf8(num).unwrap()).unwrap();
            Literal::Placeholder(ItemPlaceholder::DollarNumber(value))
        }),
    ))(i)
}

// Parse a list of values (e.g., for INSERT syntax).
pub fn value_list(i: &[u8]) -> IResult<&[u8], Vec<Literal>> {
    many0(delimited(multispace0, literal, opt(ws_sep_comma)))(i)
}

// Parse a reference to a named schema.table, with an optional alias
pub fn schema_table_reference(i: &[u8]) -> IResult<&[u8], String> {
    map(sql_identifier, |tup| {
        String::from(str::from_utf8(tup).unwrap())
    })(i)
}

// Parse rule for a comment part.
pub fn parse_comment(i: &[u8]) -> IResult<&[u8], Literal> {
    preceded(
        delimited(multispace0, tag_no_case("comment"), multispace1),
        string_literal,
    )(i)
}
