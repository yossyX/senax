use nom::branch::alt;
use nom::bytes::complete::{tag, tag_no_case};
use nom::character::complete::{alphanumeric1, multispace0, multispace1};
use nom::combinator::{map, opt};
use nom::multi::separated_list0;
use nom::sequence::{delimited, preceded, tuple};
use nom::IResult;
use serde::{Deserialize, Serialize};
use std::fmt;

use super::common::{integer_literal, string_literal, ws_sep_comma, ws_sep_equals};
use crate::common::{sql_identifier, Literal};

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableOption {
    Comment(Literal),
    Collation(String),
    Engine(String),
    Another,
}

impl fmt::Display for TableOption {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TableOption::Comment(ref comment) => write!(f, "COMMENT={}", comment.to_string()),
            TableOption::Collation(ref collation) => write!(f, "COLLATE={}", collation),
            TableOption::Engine(ref engine) => write!(f, "ENGINE={}", engine),
            TableOption::Another => Ok(()),
        }
    }
}

pub fn table_options(i: &[u8]) -> IResult<&[u8], Vec<TableOption>> {
    separated_list0(table_options_separator, create_option)(i)
}

fn table_options_separator(i: &[u8]) -> IResult<&[u8], ()> {
    map(alt((multispace1, ws_sep_comma)), |_| ())(i)
}

fn create_option(i: &[u8]) -> IResult<&[u8], TableOption> {
    alt((
        create_option_type,
        create_option_pack_keys,
        create_option_engine,
        create_option_auto_increment,
        create_option_default_charset,
        create_option_collate,
        create_option_quoted_collate,
        create_option_comment,
        create_option_max_rows,
        create_option_avg_row_length,
        create_option_row_format,
        create_option_key_block_size,
    ))(i)
}

/// Helper to parse equals-separated create option pairs.
/// Throws away the create option and value
pub fn create_option_equals_pair<'a, I, O1, O2, F, G>(
    mut first: F,
    mut second: G,
) -> impl FnMut(I) -> IResult<I, TableOption>
where
    F: FnMut(I) -> IResult<I, O1>,
    G: FnMut(I) -> IResult<I, O2>,
    I: nom::InputTakeAtPosition + nom::InputTake + nom::Compare<&'a str>,
    <I as nom::InputTakeAtPosition>::Item: nom::AsChar + Clone,
{
    move |i: I| {
        let (i, _o1) = first(i)?;
        let (i, _) = ws_sep_equals(i)?;
        let (i, _o2) = second(i)?;
        Ok((i, TableOption::Another))
    }
}

fn create_option_type(i: &[u8]) -> IResult<&[u8], TableOption> {
    create_option_equals_pair(tag_no_case("type"), alphanumeric1)(i)
}

fn create_option_pack_keys(i: &[u8]) -> IResult<&[u8], TableOption> {
    create_option_equals_pair(tag_no_case("pack_keys"), alt((tag("0"), tag("1"))))(i)
}

fn create_option_engine(i: &[u8]) -> IResult<&[u8], TableOption> {
    map(
        preceded(
            delimited(multispace0, tag_no_case("engine"), ws_sep_equals),
            sql_identifier,
        ),
        |s| TableOption::Engine(String::from_utf8_lossy(s).to_string()),
    )(i)
}

fn create_option_auto_increment(i: &[u8]) -> IResult<&[u8], TableOption> {
    create_option_equals_pair(tag_no_case("auto_increment"), integer_literal)(i)
}

fn create_option_default_charset(i: &[u8]) -> IResult<&[u8], TableOption> {
    create_option_equals_pair(
        tag_no_case("default charset"),
        alt((
            tag("utf8mb4"),
            tag("utf8"),
            tag("binary"),
            tag("big5"),
            tag("ucs2"),
            tag("latin1"),
        )),
    )(i)
}

fn create_option_collate(i: &[u8]) -> IResult<&[u8], TableOption> {
    map(
        preceded(
            delimited(multispace0, tag_no_case("collate"), ws_sep_equals),
            sql_identifier,
        ),
        |s| TableOption::Collation(String::from_utf8_lossy(s).to_string()),
    )(i)
}

fn create_option_quoted_collate(i: &[u8]) -> IResult<&[u8], TableOption> {
    map(
        preceded(
            delimited(multispace0, tag_no_case("collate"), ws_sep_equals),
            string_literal,
        ),
        |s| TableOption::Collation(s.to_raw_string()),
    )(i)
}

fn create_option_comment(i: &[u8]) -> IResult<&[u8], TableOption> {
    map(
        preceded(
            delimited(multispace0, tag_no_case("comment"), ws_sep_equals),
            string_literal,
        ),
        TableOption::Comment,
    )(i)
}

fn create_option_max_rows(i: &[u8]) -> IResult<&[u8], TableOption> {
    create_option_equals_pair(tag_no_case("max_rows"), integer_literal)(i)
}

fn create_option_avg_row_length(i: &[u8]) -> IResult<&[u8], TableOption> {
    create_option_equals_pair(tag_no_case("avg_row_length"), integer_literal)(i)
}

fn create_option_row_format(i: &[u8]) -> IResult<&[u8], TableOption> {
    let (remaining_input, (_, _, _, _, _)) = tuple((
        tag_no_case("row_format"),
        multispace0,
        opt(tag("=")),
        multispace0,
        alt((
            tag_no_case("DEFAULT"),
            tag_no_case("DYNAMIC"),
            tag_no_case("FIXED"),
            tag_no_case("COMPRESSED"),
            tag_no_case("REDUNDANT"),
            tag_no_case("COMPACT"),
        )),
    ))(i)?;
    Ok((remaining_input, TableOption::Another))
}

fn create_option_key_block_size(i: &[u8]) -> IResult<&[u8], TableOption> {
    let (remaining_input, (_, _, _, _, _)) = tuple((
        tag_no_case("key_block_size"),
        multispace0,
        opt(tag("=")),
        multispace0,
        integer_literal,
    ))(i)?;
    Ok((remaining_input, TableOption::Another))
}
