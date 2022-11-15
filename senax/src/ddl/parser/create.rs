// https://github.com/ms705/nom-sql

use nom::character::complete::{digit1, multispace0, multispace1};
use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use std::str;
use std::str::FromStr;

use super::column::{Column, ColumnConstraint, ColumnSpecification};
use super::common::{
    column_identifier_no_alias, column_identifier_query, parse_comment, reference_option,
    schema_table_reference, sql_identifier, statement_terminator, type_identifier, ws_sep_comma,
    Literal, Real, SqlType, TableKey,
};
use super::create_table_options::table_options;
use super::keywords::escape;
use super::order::{order_type, OrderType};
use nom::branch::alt;
use nom::bytes::complete::{tag, tag_no_case, take_until};
use nom::combinator::{map, opt};
use nom::multi::{many0, many1};
use nom::sequence::{delimited, preceded, terminated, tuple};
use nom::IResult;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTableStatement {
    pub table: String,
    pub fields: Vec<ColumnSpecification>,
    pub keys: Option<Vec<TableKey>>,
}

impl fmt::Display for CreateTableStatement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CREATE TABLE {} ", escape(&self.table))?;
        write!(f, "(")?;
        write!(
            f,
            "{}",
            self.fields
                .iter()
                .map(|field| format!("{}", field))
                .collect::<Vec<_>>()
                .join(", ")
        )?;
        if let Some(ref keys) = self.keys {
            write!(
                f,
                ", {}",
                keys.iter()
                    .map(|key| format!("{}", key))
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
        }
        write!(f, ")")
    }
}

// MySQL grammar element for index column definition (§13.1.18, index_col_name)
pub fn index_col_name(i: &[u8]) -> IResult<&[u8], (Column, Option<OrderType>)> {
    let (remaining_input, (column, order)) = tuple((
        terminated(
            alt((column_identifier_no_alias, column_identifier_query)),
            multispace0,
        ),
        opt(order_type),
    ))(i)?;

    Ok((remaining_input, (column, order)))
}

// Helper for list of index columns
pub fn index_col_list(i: &[u8]) -> IResult<&[u8], Vec<Column>> {
    many0(map(
        terminated(index_col_name, opt(ws_sep_comma)),
        // XXX(malte): ignores length and order
        |e| e.0,
    ))(i)
}

// Parse rule for an individual key specification.
pub fn key_specification(i: &[u8]) -> IResult<&[u8], TableKey> {
    alt((
        full_text_key,
        primary_key,
        unique,
        key_or_index,
        spatial,
        constraint,
    ))(i)
}

fn full_text_key(i: &[u8]) -> IResult<&[u8], TableKey> {
    let (remaining_input, (_, _, _, _, name, _, columns, _, parser, _)) = tuple((
        tag_no_case("fulltext"),
        multispace1,
        alt((tag_no_case("key"), tag_no_case("index"))),
        multispace1,
        sql_identifier,
        multispace0,
        delimited(
            tag("("),
            delimited(multispace0, index_col_list, multispace0),
            tag(")"),
        ),
        multispace0,
        opt(delimited(
            tag("/*!50100 WITH PARSER"),
            delimited(multispace0, sql_identifier, multispace0),
            tag("*/"),
        )),
        multispace0,
    ))(i)?;

    let name = String::from_utf8(name.to_vec()).unwrap();
    let parser = parser.map(|v| String::from_utf8(v.to_vec()).unwrap());
    Ok((
        remaining_input,
        TableKey::FulltextKey(name, columns, parser),
    ))
}

fn primary_key(i: &[u8]) -> IResult<&[u8], TableKey> {
    let (remaining_input, (_, _, columns, _, _, _)) = tuple((
        tag_no_case("primary key"),
        multispace0,
        delimited(
            tag("("),
            delimited(multispace0, index_col_list, multispace0),
            tag(")"),
        ),
        opt(map(
            preceded(multispace1, tag_no_case("auto_increment")),
            |_| (),
        )),
        multispace0,
        opt(tag_no_case("USING BTREE")),
    ))(i)?;

    Ok((remaining_input, TableKey::PrimaryKey(columns)))
}

fn unique(i: &[u8]) -> IResult<&[u8], TableKey> {
    // TODO: add branching to correctly parse whitespace after `unique`
    let (remaining_input, (_, _, _, name, _, columns, _, _)) = tuple((
        tag_no_case("unique"),
        opt(preceded(
            multispace1,
            alt((tag_no_case("key"), tag_no_case("index"))),
        )),
        multispace0,
        sql_identifier,
        multispace0,
        delimited(
            tag("("),
            delimited(multispace0, index_col_list, multispace0),
            tag(")"),
        ),
        multispace0,
        opt(tag_no_case("USING BTREE")),
    ))(i)?;

    let n = String::from_utf8(name.to_vec()).unwrap();
    Ok((remaining_input, TableKey::UniqueKey(n, columns)))
}

fn key_or_index(i: &[u8]) -> IResult<&[u8], TableKey> {
    let (remaining_input, (_, _, name, _, columns, _, _)) = tuple((
        alt((tag_no_case("key"), tag_no_case("index"))),
        multispace0,
        sql_identifier,
        multispace0,
        delimited(
            tag("("),
            delimited(multispace0, index_col_list, multispace0),
            tag(")"),
        ),
        multispace0,
        opt(tag_no_case("USING BTREE")),
    ))(i)?;

    let n = String::from_utf8(name.to_vec()).unwrap();
    Ok((remaining_input, TableKey::Key(n, columns)))
}

fn spatial(i: &[u8]) -> IResult<&[u8], TableKey> {
    let (remaining_input, (_, _, _, name, _, columns)) = tuple((
        tag_no_case("spatial"),
        opt(preceded(
            multispace1,
            alt((tag_no_case("key"), tag_no_case("index"))),
        )),
        multispace0,
        sql_identifier,
        multispace0,
        delimited(
            tag("("),
            delimited(multispace0, index_col_list, multispace0),
            tag(")"),
        ),
    ))(i)?;

    let n = String::from_utf8(name.to_vec()).unwrap();
    Ok((remaining_input, TableKey::SpatialKey(n, columns)))
}

fn constraint(i: &[u8]) -> IResult<&[u8], TableKey> {
    let (
        remaining_input,
        (
            _,
            _,
            name,
            _,
            _,
            _,
            columns,
            _,
            _,
            _,
            table,
            _,
            foreign,
            on_delete,
            on_update,
            on_delete2,
        ),
    ) = tuple((
        tag_no_case("CONSTRAINT"),
        multispace1,
        sql_identifier,
        multispace1,
        tag_no_case("FOREIGN KEY"),
        multispace0,
        delimited(
            tag("("),
            delimited(multispace0, index_col_list, multispace0),
            tag(")"),
        ),
        multispace1,
        tag_no_case("REFERENCES"),
        multispace1,
        sql_identifier,
        multispace0,
        delimited(
            tag("("),
            delimited(multispace0, index_col_list, multispace0),
            tag(")"),
        ),
        opt(tuple((
            multispace1,
            tag_no_case("ON DELETE"),
            multispace1,
            reference_option,
        ))),
        opt(tuple((
            multispace1,
            tag_no_case("ON UPDATE"),
            multispace1,
            reference_option,
        ))),
        opt(tuple((
            multispace1,
            tag_no_case("ON DELETE"),
            multispace1,
            reference_option,
        ))),
    ))(i)?;

    let name = String::from_utf8(name.to_vec()).unwrap();
    let table = String::from_utf8(table.to_vec()).unwrap();
    let on_delete = if let Some(on_delete) = on_delete {
        let (_, _, _, on_delete) = on_delete;
        Some(on_delete)
    } else if let Some(on_delete) = on_delete2 {
        let (_, _, _, on_delete) = on_delete;
        Some(on_delete)
    } else {
        None
    };
    let on_update = if let Some(on_update) = on_update {
        let (_, _, _, on_update) = on_update;
        Some(on_update)
    } else {
        None
    };
    Ok((
        remaining_input,
        TableKey::Constraint(name, columns, table, foreign, on_delete, on_update),
    ))
}

// Parse rule for a comma-separated list.
pub fn key_specification_list(i: &[u8]) -> IResult<&[u8], Vec<TableKey>> {
    many1(terminated(key_specification, opt(ws_sep_comma)))(i)
}

fn field_specification(i: &[u8]) -> IResult<&[u8], ColumnSpecification> {
    let (remaining_input, (column, field_type, constraints, comment, _)) = tuple((
        column_identifier_no_alias,
        opt(delimited(multispace1, type_identifier, multispace0)),
        many0(column_constraint),
        opt(parse_comment),
        opt(ws_sep_comma),
    ))(i)?;

    let sql_type = match field_type {
        None => SqlType::Text,
        Some(ref t) => t.clone(),
    };
    Ok((
        remaining_input,
        ColumnSpecification {
            column,
            sql_type,
            constraints: constraints.into_iter().flatten().collect(),
            comment,
        },
    ))
}

// Parse rule for a comma-separated list.
pub fn field_specification_list(i: &[u8]) -> IResult<&[u8], Vec<ColumnSpecification>> {
    many1(field_specification)(i)
}

// Parse rule for a column definition constraint.
pub fn column_constraint(i: &[u8]) -> IResult<&[u8], Option<ColumnConstraint>> {
    let not_null = map(
        delimited(multispace0, tag_no_case("not null"), multispace0),
        |_| Some(ColumnConstraint::NotNull),
    );
    let null = map(
        delimited(multispace0, tag_no_case("null"), multispace0),
        |_| None,
    );
    let auto_increment = map(
        delimited(multispace0, tag_no_case("auto_increment"), multispace0),
        |_| Some(ColumnConstraint::AutoIncrement),
    );
    let primary_key = map(
        delimited(multispace0, tag_no_case("primary key"), multispace0),
        |_| Some(ColumnConstraint::PrimaryKey),
    );
    let unique = map(
        delimited(multispace0, tag_no_case("unique"), multispace0),
        |_| Some(ColumnConstraint::Unique),
    );
    let character_set = map(
        preceded(
            delimited(multispace0, tag_no_case("character set"), multispace1),
            sql_identifier,
        ),
        |cs| {
            let char_set = str::from_utf8(cs).unwrap().to_owned();
            Some(ColumnConstraint::CharacterSet(char_set))
        },
    );
    let collate = map(
        preceded(
            delimited(multispace0, tag_no_case("collate"), multispace1),
            sql_identifier,
        ),
        |c| {
            let collation = str::from_utf8(c).unwrap().to_owned();
            Some(ColumnConstraint::Collation(collation))
        },
    );
    let srid = map(
        tuple((
            multispace0,
            tag_no_case("/*!80003 SRID "),
            digit1,
            tag_no_case(" */"),
            multispace0,
        )),
        |t| Some(ColumnConstraint::Srid(super::common::len_as_u32(t.2))),
    );

    alt((
        not_null,
        null,
        auto_increment,
        default,
        primary_key,
        unique,
        character_set,
        collate,
        srid,
    ))(i)
}

fn fixed_point(i: &[u8]) -> IResult<&[u8], Literal> {
    let (remaining_input, (i, _, f)) = tuple((digit1, tag("."), digit1))(i)?;

    Ok((
        remaining_input,
        Literal::FixedPoint(Real {
            integral: i32::from_str(str::from_utf8(i).unwrap()).unwrap(),
            fractional: i32::from_str(str::from_utf8(f).unwrap()).unwrap(),
        }),
    ))
}

fn default(i: &[u8]) -> IResult<&[u8], Option<ColumnConstraint>> {
    let (remaining_input, (_, _, _, def, _)) = tuple((
        multispace0,
        tag_no_case("default"),
        multispace1,
        alt((
            map(
                delimited(tag("'"), take_until("'"), tag("'")),
                |s: &[u8]| Literal::String(String::from_utf8(s.to_vec()).unwrap()),
            ),
            fixed_point,
            map(digit1, |d| {
                let d_i64 = i64::from_str(str::from_utf8(d).unwrap()).unwrap();
                Literal::Integer(d_i64)
            }),
            map(tag("''"), |_| Literal::String(String::from(""))),
            map(tag_no_case("null"), |_| Literal::Null),
            map(
                tag_no_case("CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP"),
                |_| Literal::CurrentTimestamp,
            ),
            map(tag_no_case("current_timestamp"), |_| {
                Literal::CurrentTimestamp
            }),
        )),
        multispace0,
    ))(i)?;
    if def == Literal::Null {
        return Ok((remaining_input, None));
    }
    Ok((remaining_input, Some(ColumnConstraint::DefaultValue(def))))
}

// Parse rule for a SQL CREATE TABLE query.
// TODO(malte): support types, TEMPORARY tables, IF NOT EXISTS, AS stmt
pub fn creation(i: &[u8]) -> IResult<&[u8], CreateTableStatement> {
    let (remaining_input, (_, _, _, _, table, _, _, _, fields, _, keys, _, _, _, _, _)) = tuple((
        tag_no_case("create"),
        multispace1,
        tag_no_case("table"),
        multispace1,
        schema_table_reference,
        multispace0,
        tag("("),
        multispace0,
        field_specification_list,
        multispace0,
        opt(key_specification_list),
        multispace0,
        tag(")"),
        multispace0,
        table_options,
        statement_terminator,
    ))(i)?;
    Ok((
        remaining_input,
        CreateTableStatement {
            table,
            fields,
            keys,
        },
    ))
}
