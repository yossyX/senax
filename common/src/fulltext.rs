use nom::{
    branch::alt,
    character::complete::char,
    combinator::map,
    error::ErrorKind,
    multi::many0,
    sequence::{delimited, preceded, terminated},
    Err, IResult, InputTakeAtPosition,
};

pub enum Word {
    And(String),
    Not(String),
    Or(Vec<String>),
}

pub struct Query(pub Vec<Word>);

impl Query {
    pub fn db_query(&self) -> String {
        let mut v = Vec::new();
        for w in &self.0 {
            match w {
                Word::And(s) => {
                    if !s.is_empty() {
                        v.push(format!(r#"+"{s}""#));
                    }
                }
                Word::Not(s) => {
                    if !s.is_empty() {
                        v.push(format!(r#"-"{s}""#));
                    }
                }
                Word::Or(l) => {
                    let s = l
                        .iter()
                        .filter(|i| !i.is_empty())
                        .map(|i| format!(r#""{i}""#))
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !s.is_empty() {
                        v.push(format!(r#"+({s})"#))
                    }
                }
            }
        }
        let mut r = v.join(" ");
        r.retain(|c| !char::is_control(c) && c != '\\');
        r
    }
}

pub fn parse(i: &str) -> Query {
    let w = parse_words(i).unwrap();
    let mut v = w.1;
    if !w.0.is_empty() {
        let i = w.0.replace(|c| c == '"' || c == '(' || c == ')', " ");
        for r in parse_words(&i).unwrap().1 {
            v.push(r);
        }
    }
    Query(v)
}

fn parse_words(i: &str) -> IResult<&str, Vec<Word>> {
    preceded(space, many0(parse_query))(i)
}

fn parse_query(i: &str) -> IResult<&str, Word> {
    terminated(alt((parse_not, parse_and, parse_or, parse_word)), space)(i)
}

fn parse_word(i: &str) -> IResult<&str, Word> {
    map(alt((quoted, word)), |w| Word::And(w.to_string()))(i)
}

fn parse_and(i: &str) -> IResult<&str, Word> {
    map(preceded(char('+'), alt((quoted, word))), |w| {
        Word::And(w.to_string())
    })(i)
}

fn parse_not(i: &str) -> IResult<&str, Word> {
    map(preceded(char('-'), alt((quoted, word))), |w| {
        Word::Not(w.to_string())
    })(i)
}

fn parse_or(i: &str) -> IResult<&str, Word> {
    map(
        delimited(
            char('('),
            many0(delimited(
                space,
                alt((
                    quoted,
                    delimited(char('('), take_until_unbalanced('(', ')'), char(')')),
                    word,
                )),
                space,
            )),
            char(')'),
        ),
        |w| Word::Or(w.iter().map(|v| v.to_string()).collect()),
    )(i)
}

// https://docs.rs/parse-hyperlinks/0.23.3/src/parse_hyperlinks/lib.rs.html#41
pub fn take_until_unbalanced(
    opening_bracket: char,
    closing_bracket: char,
) -> impl Fn(&str) -> IResult<&str, &str> {
    move |i: &str| {
        let mut index = 0;
        let mut bracket_counter = 0;
        while let Some(n) = &i[index..].find(&[opening_bracket, closing_bracket][..]) {
            index += n;
            let mut it = i[index..].chars();
            match it.next().unwrap_or_default() {
                c if c == opening_bracket => {
                    bracket_counter += 1;
                    index += opening_bracket.len_utf8();
                }
                c if c == closing_bracket => {
                    // Closing bracket.
                    bracket_counter -= 1;
                    index += closing_bracket.len_utf8();
                }
                // Can not happen.
                _ => unreachable!(),
            };
            // We found the unmatched closing bracket.
            if bracket_counter == -1 {
                // We do not consume it.
                index -= closing_bracket.len_utf8();
                return Ok((&i[index..], &i[0..index]));
            };
        }
        if bracket_counter == 0 {
            Ok(("", i))
        } else {
            Err(Err::Error(nom::error::make_error(i, ErrorKind::TakeUntil)))
        }
    }
}

fn quoted(i: &str) -> IResult<&str, &str> {
    delimited(char('"'), quoted_word, char('"'))(i)
}

fn word(input: &str) -> IResult<&str, &str> {
    input.split_at_position1_complete(
        |c| char::is_whitespace(c) || c == '"' || c == '(' || c == ')',
        ErrorKind::AlphaNumeric,
    )
}

fn quoted_word(input: &str) -> IResult<&str, &str> {
    input.split_at_position_complete(|c| c == '"')
}

fn space(input: &str) -> IResult<&str, &str> {
    input.split_at_position_complete(|c| !char::is_whitespace(c))
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test() {
        assert_eq!(parse("a テスト").db_query(), r#"+"a" +"テスト""#);
        assert_eq!(parse(" a　b ").db_query(), r#"+"a" +"b""#);
        assert_eq!(parse(r#" a "bb "#).db_query(), r#"+"a" +"bb""#);
        assert_eq!(parse(r#" a (bb "#).db_query(), r#"+"a" +"bb""#);
        assert_eq!(parse("a -b").db_query(), r#"+"a" -"b""#);
        assert_eq!(parse(r#""a(a""#).db_query(), r#"+"a(a""#);
        assert_eq!(parse("a (b c)").db_query(), r#"+"a" +("b" "c")"#);
        assert_eq!(parse("a(b c)").db_query(), r#"+"a" +("b" "c")"#);
        assert_eq!(parse("a(b (c d))").db_query(), r#"+"a" +("b" "c d")"#);
        assert_eq!(parse("a () b").db_query(), r#"+"a" +"b""#);
        assert_eq!(parse(r#"a "" b"#).db_query(), r#"+"a" +"b""#);
        assert_eq!(
            parse(r#"a "cc dd)\\" b"#).db_query(),
            r#"+"a" +"cc dd)" +"b""#
        );
    }
}
