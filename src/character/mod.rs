#[macro_use]
mod macros;

pub mod streaming;
pub mod complete;

use internal::{Err, IResult, Needed};
use error::ParseError;
use ::lib::std::ops::{Range, RangeFrom, RangeTo};
use traits::{need_more, AsChar, AtEof, FindToken, InputIter, Slice};
use traits::{InputLength, InputTakeAtPosition};
use traits::{need_more_err, ParseTo, Compare, CompareResult};
use error::ErrorKind;

// backward compatible version that can handle streaming and complete versions at the same time
#[doc(hidden)]
pub fn char<I, Error: ParseError<I>>(c: char) -> impl Fn(I) -> IResult<I, char, Error>
where
  I: Slice<RangeFrom<usize>> + InputIter + AtEof,
  <I as InputIter>::Item: AsChar,
{
  move |i: I| match (i).iter_elements().next().map(|t| {
    let b = t.as_char() == c;
    (&c, b)
  }) {
    None => need_more(i, Needed::Size(1)),
    Some((_, false)) => {
      //let e: ErrorKind = ErrorKind::Char;
      Err(Err::Error(Error::from_char(i, c)))
    }
    Some((c, true)) => Ok((i.slice(c.len()..), c.as_char())),
  }
}

// backward compatible version that can handle streaming and complete versions at the same time
#[doc(hidden)]
pub fn one_of<I, T, Error: ParseError<I>>(list: T) -> impl Fn(I) -> IResult<I, char, Error>
where
  I: Slice<RangeFrom<usize>> + InputIter + AtEof,
  <I as InputIter>::Item: AsChar + Copy,
  T: FindToken<<I as InputIter>::Item>,
{
  move |i: I| match (i).iter_elements().next().map(|c| (c, list.find_token(c))) {
    None => need_more(i, Needed::Size(1)),
    Some((_, false)) => Err(Err::Error(Error::from_error_kind(i, ErrorKind::OneOf))),
    Some((c, true)) => Ok((i.slice(c.len()..), c.as_char())),
  }
}

// backward compatible version that can handle streaming and complete versions at the same time
#[doc(hidden)]
pub fn none_of<I, T, Error: ParseError<I>>(list: T) -> impl Fn(I) -> IResult<I, char, Error>
where
  I: Slice<RangeFrom<usize>> + InputIter + AtEof,
  <I as InputIter>::Item: AsChar + Copy,
  T: FindToken<<I as InputIter>::Item>,
{
  move |i: I| match (i).iter_elements().next().map(|c| (c, !list.find_token(c))) {
    None => need_more(i, Needed::Size(1)),
    Some((_, false)) => Err(Err::Error(Error::from_error_kind(i, ErrorKind::NoneOf))),
    Some((c, true)) => Ok((i.slice(c.len()..), c.as_char())),
  }
}

named!(#[doc="Matches a newline character '\\n'"], pub newline<char>, char!('\n'));

named!(#[doc="Matches a tab character '\\t'"], pub tab<char>, char!('\t'));

pub fn crlf<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Slice<Range<usize>> + Slice<RangeFrom<usize>> + Slice<RangeTo<usize>>,
  T: InputIter + AtEof,
  T: Compare<&'static str>,
{
  match input.compare("\r\n") {
    //FIXME: is this the right index?
    CompareResult::Ok => Ok((input.slice(2..), input.slice(0..2))),
    CompareResult::Incomplete => need_more_err(input, Needed::Size(2), ErrorKind::CrLf),
    CompareResult::Error => {
      let e: ErrorKind = ErrorKind::CrLf;
      Err(Err::Error(E::from_error_kind(input, e)))
    }
  }
}

// FIXME: when rust-lang/rust#17436 is fixed, macros will be able to export
// public methods
pub fn not_line_ending<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Slice<Range<usize>> + Slice<RangeFrom<usize>> + Slice<RangeTo<usize>>,
  T: InputIter + InputLength + AtEof,
  T: Compare<&'static str>,
  <T as InputIter>::Item: AsChar,
  <T as InputIter>::RawItem: AsChar,
{
  match input.position(|item| {
    let c = item.as_char();
    c == '\r' || c == '\n'
  }) {
    None => {
      if input.at_eof() {
        Ok((input.slice(input.input_len()..), input))
      } else {
        Err(Err::Incomplete(Needed::Unknown))
      }
    }
    Some(index) => {
      let mut it = input.slice(index..).iter_elements();
      let nth = it.next().unwrap().as_char();
      if nth == '\r' {
        let sliced = input.slice(index..);
        let comp = sliced.compare("\r\n");
        match comp {
          //FIXME: calculate the right index
          CompareResult::Incomplete => need_more_err(input, Needed::Unknown, ErrorKind::Tag),
          CompareResult::Error => {
            let e: ErrorKind = ErrorKind::Tag;
            Err(Err::Error(E::from_error_kind(input, e)))
          }
          CompareResult::Ok => Ok((input.slice(index..), input.slice(..index))),
        }
      } else {
        Ok((input.slice(index..), input.slice(..index)))
      }
    }
  }
}

/// Recognizes an end of line (both '\n' and '\r\n')
pub fn line_ending<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Slice<Range<usize>> + Slice<RangeFrom<usize>> + Slice<RangeTo<usize>>,
  T: InputIter + InputLength + AtEof,
  T: Compare<&'static str>,
{
  match input.compare("\n") {
    CompareResult::Ok => Ok((input.slice(1..), input.slice(0..1))),
    CompareResult::Incomplete => need_more_err(input, Needed::Size(1), ErrorKind::CrLf),
    CompareResult::Error => {
      match input.compare("\r\n") {
        //FIXME: is this the right index?
        CompareResult::Ok => Ok((input.slice(2..), input.slice(0..2))),
        CompareResult::Incomplete => need_more_err(input, Needed::Size(2), ErrorKind::CrLf),
        CompareResult::Error => Err(Err::Error(E::from_error_kind(input, ErrorKind::CrLf))),
      }
    }
  }
}

pub fn eol<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: Slice<Range<usize>> + Slice<RangeFrom<usize>> + Slice<RangeTo<usize>>,
  T: InputIter + InputLength + AtEof,
  T: Compare<&'static str>,
{
  line_ending(input)
}

/// Tests if byte is ASCII alphabetic: A-Z, a-z
#[inline]
pub fn is_alphabetic(chr: u8) -> bool {
  (chr >= 0x41 && chr <= 0x5A) || (chr >= 0x61 && chr <= 0x7A)
}

/// Tests if byte is ASCII digit: 0-9
#[inline]
pub fn is_digit(chr: u8) -> bool {
  chr >= 0x30 && chr <= 0x39
}

/// Tests if byte is ASCII hex digit: 0-9, A-F, a-f
#[inline]
pub fn is_hex_digit(chr: u8) -> bool {
  (chr >= 0x30 && chr <= 0x39) || (chr >= 0x41 && chr <= 0x46) || (chr >= 0x61 && chr <= 0x66)
}

/// Tests if byte is ASCII octal digit: 0-7
#[inline]
pub fn is_oct_digit(chr: u8) -> bool {
  chr >= 0x30 && chr <= 0x37
}

/// Tests if byte is ASCII alphanumeric: A-Z, a-z, 0-9
#[inline]
pub fn is_alphanumeric(chr: u8) -> bool {
  is_alphabetic(chr) || is_digit(chr)
}

/// Tests if byte is ASCII space or tab
#[inline]
pub fn is_space(chr: u8) -> bool {
  chr == b' ' || chr == b'\t'
}

// FIXME: when rust-lang/rust#17436 is fixed, macros will be able to export
//pub filter!(alpha is_alphabetic)
//pub filter!(digit is_digit)
//pub filter!(hex_digit is_hex_digit)
//pub filter!(oct_digit is_oct_digit)
//pub filter!(alphanumeric is_alphanumeric)

/// Recognizes one or more lowercase and uppercase alphabetic characters.
/// For ASCII strings: a-zA-Z
/// For UTF8 strings, any alphabetic code point (ie, not only the ASCII ones)
pub fn alpha<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  alpha1(input)
}

/// Recognizes zero or more lowercase and uppercase alphabetic characters.
/// For ASCII strings: a-zA-Z
/// For UTF8 strings, any alphabetic code point (ie, not only the ASCII ones)
pub fn alpha0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position(|item| !item.is_alpha())
}

/// Recognizes one or more lowercase and uppercase alphabetic characters
/// For ASCII strings: a-zA-Z
/// For UTF8 strings, any alphabetic code point (ie, not only the ASCII ones)
pub fn alpha1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position1(|item| !item.is_alpha(), ErrorKind::Alpha)
}

/// Recognizes one or more numerical characters: 0-9
pub fn digit<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  digit1(input)
}

/// Recognizes zero or more numerical characters: 0-9
pub fn digit0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position(|item| !item.is_dec_digit())
}

/// Recognizes one or more numerical characters: 0-9
pub fn digit1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position1(|item| !item.is_dec_digit(), ErrorKind::Digit)
}

/// Recognizes one or more hexadecimal numerical characters: 0-9, A-F, a-f
pub fn hex_digit<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  hex_digit1(input)
}

/// Recognizes zero or more hexadecimal numerical characters: 0-9, A-F, a-f
pub fn hex_digit0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position(|item| !item.is_hex_digit())
}
/// Recognizes one or more hexadecimal numerical characters: 0-9, A-F, a-f
pub fn hex_digit1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position1(|item| !item.is_hex_digit(), ErrorKind::HexDigit)
}

/// Recognizes one or more octal characters: 0-7
pub fn oct_digit<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  oct_digit1(input)
}

/// Recognizes zero or more octal characters: 0-7
pub fn oct_digit0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position(|item| !item.is_oct_digit())
}

/// Recognizes one or more octal characters: 0-7
pub fn oct_digit1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position1(|item| !item.is_oct_digit(), ErrorKind::OctDigit)
}

/// Recognizes one or more numerical and alphabetic characters
/// For ASCII strings: 0-9a-zA-Z
/// For UTF8 strings, 0-9 and any alphabetic code point (ie, not only the ASCII ones)
pub fn alphanumeric<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  alphanumeric1(input)
}

/// Recognizes zero or more numerical and alphabetic characters.
/// For ASCII strings: 0-9a-zA-Z
/// For UTF8 strings, 0-9 and any alphabetic code point (ie, not only the ASCII ones)
pub fn alphanumeric0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position(|item| !item.is_alphanum())
}
/// Recognizes one or more numerical and alphabetic characters.
/// For ASCII strings: 0-9a-zA-Z
/// For UTF8 strings, 0-9 and any alphabetic code point (ie, not only the ASCII ones)
pub fn alphanumeric1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar,
{
  input.split_at_position1(|item| !item.is_alphanum(), ErrorKind::AlphaNumeric)
}

/// Recognizes one or more spaces and tabs
pub fn space<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar + Clone,
{
  space1(input)
}

/// Recognizes zero or more spaces and tabs
pub fn space0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar + Clone,
{
  input.split_at_position(|item| {
    let c = item.clone().as_char();
    !(c == ' ' || c == '\t')
  })
}
/// Recognizes one or more spaces and tabs
pub fn space1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar + Clone,
{
  input.split_at_position1(
    |item| {
      let c = item.clone().as_char();
      !(c == ' ' || c == '\t')
    },
    ErrorKind::Space,
  )
}

/// Recognizes one or more spaces, tabs, carriage returns and line feeds
pub fn multispace<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar + Clone,
{
  multispace1(input)
}

/// Recognizes zero or more spaces, tabs, carriage returns and line feeds
pub fn multispace0<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar + Clone,
{
  input.split_at_position(|item| {
    let c = item.clone().as_char();
    !(c == ' ' || c == '\t' || c == '\r' || c == '\n')
  })
}
/// Recognizes one or more spaces, tabs, carriage returns and line feeds
pub fn multispace1<T, E: ParseError<T>>(input: T) -> IResult<T, T, E>
where
  T: InputTakeAtPosition,
  <T as InputTakeAtPosition>::Item: AsChar + Clone,
{
  input.split_at_position1(
    |item| {
      let c = item.clone().as_char();
      !(c == ' ' || c == '\t' || c == '\r' || c == '\n')
    },
    ErrorKind::MultiSpace,
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use internal::{Err, IResult, Needed};
  use error::ParseError;
  use types::{CompleteByteSlice, CompleteStr};


  macro_rules! assert_parse(
    ($left: expr, $right: expr) => {
      let res: $crate::IResult<_, _, (_, ErrorKind)> = $left;
      assert_eq!(res, $right);
    };
  );

  #[test]
  fn character() {
    let empty: &[u8] = b"";
    let a: &[u8] = b"abcd";
    let b: &[u8] = b"1234";
    let c: &[u8] = b"a123";
    let d: &[u8] = "azé12".as_bytes();
    let e: &[u8] = b" ";
    let f: &[u8] = b" ;";
    //assert_eq!(alpha::<_, (_, ErrorKind)>(a), Err(Err::Incomplete(Needed::Size(1))));
    assert_parse!(alpha(a), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      alpha::<_, (_, ErrorKind)>(CompleteByteSlice(a)),
      Ok((CompleteByteSlice(empty), CompleteByteSlice(a)))
    );
    assert_eq!(
      alpha(b),
      Err(Err::Error((b, ErrorKind::Alpha)))
    );
    assert_eq!(alpha::<_, (_, ErrorKind)>(c), Ok((&c[1..], &b"a"[..])));
    assert_eq!(alpha::<_, (_, ErrorKind)>(d), Ok(("é12".as_bytes(), &b"az"[..])));
    assert_eq!(
      digit(a),
      Err(Err::Error((a, ErrorKind::Digit)))
    );
    assert_eq!(digit::<_, (_, ErrorKind)>(b), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      digit::<_, (_, ErrorKind)>(CompleteByteSlice(b)),
      Ok((CompleteByteSlice(empty), CompleteByteSlice(b)))
    );
    assert_eq!(
      digit(c),
      Err(Err::Error((c, ErrorKind::Digit)))
    );
    assert_eq!(
      digit(d),
      Err(Err::Error((d, ErrorKind::Digit)))
    );
    assert_eq!(hex_digit::<_, (_, ErrorKind)>(a), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      hex_digit::<_, (_, ErrorKind)>(CompleteByteSlice(a)),
      Ok((CompleteByteSlice(empty), CompleteByteSlice(a)))
    );
    assert_eq!(hex_digit::<_, (_, ErrorKind)>(b), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      hex_digit::<_, (_, ErrorKind)>(CompleteByteSlice(b)),
      Ok((CompleteByteSlice(empty), CompleteByteSlice(b)))
    );
    assert_eq!(hex_digit::<_, (_, ErrorKind)>(c), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      hex_digit::<_, (_, ErrorKind)>(CompleteByteSlice(c)),
      Ok((CompleteByteSlice(empty), CompleteByteSlice(c)))
    );
    assert_eq!(hex_digit::<_, (_, ErrorKind)>(d), Ok(("zé12".as_bytes(), &b"a"[..])));
    assert_eq!(
      hex_digit(e),
      Err(Err::Error((e, ErrorKind::HexDigit)))
    );
    assert_eq!(
      oct_digit(a),
      Err(Err::Error((a, ErrorKind::OctDigit)))
    );
    assert_eq!(oct_digit::<_, (_, ErrorKind)>(b), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      oct_digit::<_, (_, ErrorKind)>(CompleteByteSlice(b)),
      Ok((CompleteByteSlice(empty), CompleteByteSlice(b)))
    );
    assert_eq!(
      oct_digit(c),
      Err(Err::Error((c, ErrorKind::OctDigit)))
    );
    assert_eq!(
      oct_digit(d),
      Err(Err::Error((d, ErrorKind::OctDigit)))
    );
    assert_eq!(alphanumeric::<_, (_, ErrorKind)>(a), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      alphanumeric::<_, (_, ErrorKind)>(CompleteByteSlice(a)),
      Ok((CompleteByteSlice(empty), CompleteByteSlice(a)))
    );
    //assert_eq!(fix_error!(b,(), alphanumeric), Ok((empty, b)));
    assert_eq!(alphanumeric::<_, (_, ErrorKind)>(c), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      alphanumeric::<_, (_, ErrorKind)>(CompleteByteSlice(c)),
      Ok((CompleteByteSlice(empty), CompleteByteSlice(c)))
    );
    assert_eq!(alphanumeric::<_, (_, ErrorKind)>(d), Ok(("é12".as_bytes(), &b"az"[..])));
    assert_eq!(space::<_, (_, ErrorKind)>(e), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      space::<_, (_, ErrorKind)>(CompleteByteSlice(e)),
      Ok((CompleteByteSlice(empty), CompleteByteSlice(b" ")))
    );
    assert_eq!(space::<_, (_, ErrorKind)>(f), Ok((&b";"[..], &b" "[..])));
    assert_eq!(
      space::<_, (_, ErrorKind)>(CompleteByteSlice(f)),
      Ok((CompleteByteSlice(b";"), CompleteByteSlice(b" ")))
    );
  }

  #[cfg(feature = "alloc")]
  #[test]
  fn character_s() {
    let empty = "";
    let a = "abcd";
    let b = "1234";
    let c = "a123";
    let d = "azé12";
    let e = " ";
    assert_eq!(alpha::<_, (_, ErrorKind)>(a), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      alpha::<_, (_, ErrorKind)>(CompleteStr(a)),
      Ok((CompleteStr(empty), CompleteStr(a)))
    );
    assert_eq!(
      alpha(b),
      Err(Err::Error((b, ErrorKind::Alpha)))
    );
    assert_eq!(alpha::<_, (_, ErrorKind)>(c), Ok((&c[1..], &"a"[..])));
    assert_eq!(alpha::<_, (_, ErrorKind)>(d), Ok(("12", &"azé"[..])));
    assert_eq!(
      digit(a),
      Err(Err::Error((a, ErrorKind::Digit)))
    );
    assert_eq!(digit::<_, (_, ErrorKind)>(b), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      digit::<_, (_, ErrorKind)>(CompleteStr(b)),
      Ok((CompleteStr(empty), CompleteStr(b)))
    );
    assert_eq!(
      digit(c),
      Err(Err::Error((c, ErrorKind::Digit)))
    );
    assert_eq!(
      digit(d),
      Err(Err::Error((d, ErrorKind::Digit)))
    );
    assert_eq!(hex_digit::<_, (_, ErrorKind)>(a), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      hex_digit::<_, (_, ErrorKind)>(CompleteStr(a)),
      Ok((CompleteStr(empty), CompleteStr(a)))
    );
    assert_eq!(hex_digit::<_, (_, ErrorKind)>(b), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      hex_digit::<_, (_, ErrorKind)>(CompleteStr(b)),
      Ok((CompleteStr(empty), CompleteStr(b)))
    );
    assert_eq!(hex_digit::<_, (_, ErrorKind)>(c), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      hex_digit::<_, (_, ErrorKind)>(CompleteStr(c)),
      Ok((CompleteStr(empty), CompleteStr(c)))
    );
    assert_eq!(hex_digit::<_, (_, ErrorKind)>(d), Ok(("zé12", &"a"[..])));
    assert_eq!(
      hex_digit(e),
      Err(Err::Error((e, ErrorKind::HexDigit)))
    );
    assert_eq!(
      oct_digit(a),
      Err(Err::Error((a, ErrorKind::OctDigit)))
    );
    assert_eq!(oct_digit::<_, (_, ErrorKind)>(b), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      oct_digit::<_, (_, ErrorKind)>(CompleteStr(b)),
      Ok((CompleteStr(empty), CompleteStr(b)))
    );
    assert_eq!(
      oct_digit(c),
      Err(Err::Error((c, ErrorKind::OctDigit)))
    );
    assert_eq!(
      oct_digit(d),
      Err(Err::Error((d, ErrorKind::OctDigit)))
    );
    assert_eq!(alphanumeric::<_, (_, ErrorKind)>(a), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      alphanumeric::<_, (_, ErrorKind)>(CompleteStr(a)),
      Ok((CompleteStr(empty), CompleteStr(a)))
    );
    //assert_eq!(fix_error!(b,(), alphanumeric), Ok((empty, b)));
    assert_eq!(alphanumeric::<_, (_, ErrorKind)>(c), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      alphanumeric::<_, (_, ErrorKind)>(CompleteStr(c)),
      Ok((CompleteStr(empty), CompleteStr(c)))
    );
    assert_eq!(alphanumeric::<_, (_, ErrorKind)>(d), Err(Err::Incomplete(Needed::Size(1))));
    assert_eq!(
      alphanumeric::<_, (_, ErrorKind)>(CompleteStr(d)),
      Ok((CompleteStr(""), CompleteStr("azé12")))
    );
    assert_eq!(space::<_, (_, ErrorKind)>(e), Err(Err::Incomplete(Needed::Size(1))));
  }

  use traits::Offset;
  #[test]
  fn offset() {
    let a = &b"abcd;"[..];
    let b = &b"1234;"[..];
    let c = &b"a123;"[..];
    let d = &b" \t;"[..];
    let e = &b" \t\r\n;"[..];
    let f = &b"123abcDEF;"[..];

    match alpha::<_, (_, ErrorKind)>(a) {
      Ok((i, _)) => {
        assert_eq!(a.offset(i) + i.len(), a.len());
      }
      _ => panic!("wrong return type in offset test for alpha"),
    }
    match digit::<_, (_, ErrorKind)>(b) {
      Ok((i, _)) => {
        assert_eq!(b.offset(i) + i.len(), b.len());
      }
      _ => panic!("wrong return type in offset test for digit"),
    }
    match alphanumeric::<_, (_, ErrorKind)>(c) {
      Ok((i, _)) => {
        assert_eq!(c.offset(i) + i.len(), c.len());
      }
      _ => panic!("wrong return type in offset test for alphanumeric"),
    }
    match space::<_, (_, ErrorKind)>(d) {
      Ok((i, _)) => {
        assert_eq!(d.offset(i) + i.len(), d.len());
      }
      _ => panic!("wrong return type in offset test for space"),
    }
    match multispace::<_, (_, ErrorKind)>(e) {
      Ok((i, _)) => {
        assert_eq!(e.offset(i) + i.len(), e.len());
      }
      _ => panic!("wrong return type in offset test for multispace"),
    }
    match hex_digit::<_, (_, ErrorKind)>(f) {
      Ok((i, _)) => {
        assert_eq!(f.offset(i) + i.len(), f.len());
      }
      _ => panic!("wrong return type in offset test for hex_digit"),
    }
    match oct_digit::<_, (_, ErrorKind)>(f) {
      Ok((i, _)) => {
        assert_eq!(f.offset(i) + i.len(), f.len());
      }
      _ => panic!("wrong return type in offset test for oct_digit"),
    }
  }

  #[test]
  fn is_not_line_ending_bytes() {
    let a: &[u8] = b"ab12cd\nefgh";
    assert_eq!(not_line_ending::<_, (_, ErrorKind)>(a), Ok((&b"\nefgh"[..], &b"ab12cd"[..])));

    let b: &[u8] = b"ab12cd\nefgh\nijkl";
    assert_eq!(
      not_line_ending::<_, (_, ErrorKind)>(b),
      Ok((&b"\nefgh\nijkl"[..], &b"ab12cd"[..]))
    );

    let c: &[u8] = b"ab12cd\r\nefgh\nijkl";
    assert_eq!(
      not_line_ending::<_, (_, ErrorKind)>(c),
      Ok((&b"\r\nefgh\nijkl"[..], &b"ab12cd"[..]))
    );

    let d = CompleteByteSlice(b"ab12cd");
    assert_eq!(not_line_ending::<_, (_, ErrorKind)>(d), Ok((CompleteByteSlice(b""), d)));

    let d: &[u8] = b"ab12cd";
    assert_eq!(not_line_ending::<_, (_, ErrorKind)>(d), Err(Err::Incomplete(Needed::Unknown)));
  }

  #[test]
  fn is_not_line_ending_str() {
    /*
    let a: &str = "ab12cd\nefgh";
    assert_eq!(not_line_ending(a), Ok((&"\nefgh"[..], &"ab12cd"[..])));

    let b: &str = "ab12cd\nefgh\nijkl";
    assert_eq!(not_line_ending(b), Ok((&"\nefgh\nijkl"[..], &"ab12cd"[..])));

    let c: &str = "ab12cd\r\nefgh\nijkl";
    assert_eq!(not_line_ending(c), Ok((&"\r\nefgh\nijkl"[..], &"ab12cd"[..])));

    let d = "βèƒôřè\nÂßÇáƒƭèř";
    assert_eq!(not_line_ending(d), Ok((&"\nÂßÇáƒƭèř"[..], &"βèƒôřè"[..])));

    let e = "βèƒôřè\r\nÂßÇáƒƭèř";
    assert_eq!(not_line_ending(e), Ok((&"\r\nÂßÇáƒƭèř"[..], &"βèƒôřè"[..])));
    */

    let f = "βèƒôřè\rÂßÇáƒƭèř";
    assert_eq!(
      not_line_ending(f),
      Err(Err::Error((f, ErrorKind::Tag)))
    );

    let g = CompleteStr("ab12cd");
    assert_eq!(not_line_ending::<_, (_, ErrorKind)>(g), Ok((CompleteStr(""), g)));

    let g2: &str = "ab12cd";
    assert_eq!(not_line_ending::<_, (_, ErrorKind)>(g2), Err(Err::Incomplete(Needed::Unknown)));
  }

  #[test]
  fn hex_digit_test() {
    let i = &b"0123456789abcdefABCDEF;"[..];
    assert_parse!(hex_digit(i), Ok((&b";"[..], &i[..i.len() - 1])));

    let i = &b"g"[..];
    assert_parse!(
      hex_digit(i),
      Err(Err::Error(error_position!(i, ErrorKind::HexDigit)))
    );

    let i = &b"G"[..];
    assert_parse!(
      hex_digit(i),
      Err(Err::Error(error_position!(i, ErrorKind::HexDigit)))
    );

    assert!(is_hex_digit(b'0'));
    assert!(is_hex_digit(b'9'));
    assert!(is_hex_digit(b'a'));
    assert!(is_hex_digit(b'f'));
    assert!(is_hex_digit(b'A'));
    assert!(is_hex_digit(b'F'));
    assert!(!is_hex_digit(b'g'));
    assert!(!is_hex_digit(b'G'));
    assert!(!is_hex_digit(b'/'));
    assert!(!is_hex_digit(b':'));
    assert!(!is_hex_digit(b'@'));
    assert!(!is_hex_digit(b'\x60'));
  }

  #[test]
  fn oct_digit_test() {
    let i = &b"01234567;"[..];
    assert_parse!(oct_digit(i), Ok((&b";"[..], &i[..i.len() - 1])));

    let i = &b"8"[..];
    assert_parse!(
      oct_digit(i),
      Err(Err::Error(error_position!(i, ErrorKind::OctDigit)))
    );

    assert!(is_oct_digit(b'0'));
    assert!(is_oct_digit(b'7'));
    assert!(!is_oct_digit(b'8'));
    assert!(!is_oct_digit(b'9'));
    assert!(!is_oct_digit(b'a'));
    assert!(!is_oct_digit(b'A'));
    assert!(!is_oct_digit(b'/'));
    assert!(!is_oct_digit(b':'));
    assert!(!is_oct_digit(b'@'));
    assert!(!is_oct_digit(b'\x60'));
  }

  #[test]
  fn full_line_windows() {
    named!(
      take_full_line<(&[u8], &[u8])>,
      tuple!(not_line_ending, line_ending)
    );
    let input = b"abc\r\n";
    let output = take_full_line(input);
    assert_eq!(output, Ok((&b""[..], (&b"abc"[..], &b"\r\n"[..]))));
  }

  #[test]
  fn full_line_unix() {
    named!(
      take_full_line<(&[u8], &[u8])>,
      tuple!(not_line_ending, line_ending)
    );
    let input = b"abc\n";
    let output = take_full_line(input);
    assert_eq!(output, Ok((&b""[..], (&b"abc"[..], &b"\n"[..]))));
  }

  #[test]
  fn check_windows_lineending() {
    let input = b"\r\n";
    let output = line_ending(&input[..]);
    assert_parse!(output, Ok((&b""[..], &b"\r\n"[..])));
  }

  #[test]
  fn check_unix_lineending() {
    let input = b"\n";
    let output = line_ending(&input[..]);
    assert_parse!(output, Ok((&b""[..], &b"\n"[..])));
  }

  #[test]
  fn cr_lf() {
    assert_parse!(crlf(&b"\r\na"[..]), Ok((&b"a"[..], &b"\r\n"[..])));
    assert_parse!(crlf(&b"\r"[..]), Err(Err::Incomplete(Needed::Size(2))));
    assert_parse!(
      crlf(&b"\ra"[..]),
      Err(Err::Error(error_position!(&b"\ra"[..], ErrorKind::CrLf)))
    );

    assert_parse!(crlf("\r\na"), Ok(("a", "\r\n")));
    assert_parse!(crlf("\r"), Err(Err::Incomplete(Needed::Size(2))));
    assert_parse!(
      crlf("\ra"),
      Err(Err::Error(error_position!("\ra", ErrorKind::CrLf)))
    );
  }

  #[test]
  fn end_of_line() {
    assert_parse!(eol(&b"\na"[..]), Ok((&b"a"[..], &b"\n"[..])));
    assert_parse!(eol(&b"\r\na"[..]), Ok((&b"a"[..], &b"\r\n"[..])));
    assert_parse!(eol(&b"\r"[..]), Err(Err::Incomplete(Needed::Size(2))));
    assert_parse!(
      eol(&b"\ra"[..]),
      Err(Err::Error(error_position!(&b"\ra"[..], ErrorKind::CrLf)))
    );

    assert_parse!(eol("\na"), Ok(("a", "\n")));
    assert_parse!(eol("\r\na"), Ok(("a", "\r\n")));
    assert_parse!(eol("\r"), Err(Err::Incomplete(Needed::Size(2))));
    assert_parse!(
      eol("\ra"),
      Err(Err::Error(error_position!("\ra", ErrorKind::CrLf)))
    );
  }

  #[allow(dead_code)]
  pub fn end_of_line_completestr(input: CompleteStr) -> IResult<CompleteStr, CompleteStr> {
    alt!(input, eof!() | eol)
  }
}