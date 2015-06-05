/// Copyright (c) 2015, Takeru Ohta <phjgt308@gmail.com>
//
extern crate eetf;

use eetf::*;
use std::io::Cursor;

macro_rules! assert_decode {
    ($x:expr, $y:expr) => {{
        let mut cur: Cursor<&[u8]> = Cursor::new(&$y);
        assert_eq!($x, decode(&mut cur))
    }}
}

#[test]
fn decode_empty() {
    let input = [];
    let expected = None;
    assert_decode!(expected, input);
}

#[test]
fn decode_small_integer() {
    let input = [131,97,5];
    let expected = 5;
    assert_decode!(Some(Term::Int(expected)), input);
}

#[test]
fn decode_integer() {
    let input = [131,98,0,0,4,210];
    let expected = 1234;
    assert_decode!(Some(Term::Int(expected)), input);

    let input = [131,98,255,255,251,46];
    let expected = -1234;
    assert_decode!(Some(Term::Int(expected)), input);
}

#[test]
fn decode_atom() {
    let input = [131,100,0,4,104,111,103,101];
    let expected = "hoge".to_string();
    assert_decode!(Some(Term::Atom(expected)), input);
}

#[test]
fn decode_small_atom() {
    let input = [131,115,4,104,111,103,101];
    let expected = "hoge".to_string();
    assert_decode!(Some(Term::Atom(expected)), input);
}

#[test]
fn decode_atom_utf8() {
    let input = [131,118,0,4,104,111,103,101];
    let expected = "hoge".to_string();
    assert_decode!(Some(Term::Atom(expected)), input);
}

#[test]
fn decode_small_atom_utf8() {
    let input = [131,119,4,104,111,103,101];
    let expected = "hoge".to_string();
    assert_decode!(Some(Term::Atom(expected)), input);
}

#[test]
fn decode_small_tuple() {
    let input = [131,104,2,97,1,100,0,3,111,110,101];
    let expected = vec![Term::Int(1), Term::Atom("one".to_string())];
    assert_decode!(Some(Term::Tuple(expected)), input);
}

#[test]
fn decode_large_tuple() {
    let input = [131,105,0,0,0,2,97,1,100,0,3,111,110,101];
    let expected = vec![Term::Int(1), Term::Atom("one".to_string())];
    assert_decode!(Some(Term::Tuple(expected)), input);
}

#[test]
fn decode_nil() {
    let input = [131,106];
    let expected = vec![];
    assert_decode!(Some(Term::List(expected)), input);
}

#[test]
fn decode_binary() {
    let input = [131,109,0,0,0,4,104,111,103,101];
    let expected = vec![104,111,103,101]; // "hoge"
    assert_decode!(Some(Term::Binary(expected)), input);
}

// #[test]
// fn decode_float() {
//     let input = [131,99,49,46,50,51,51,57,57,57,57,57,57,57,57,57,57,57,57,57,56,53,55,57,101,43,48,49,0,0,0,0,0];
//     let expected = 12.34;
//     assert_eq!(Term::Float(expected), decode(&input).unwrap());
// }

// #[test]
// fn decode_new_float() {
//     let input = [131,70,64,40,174,20,122,225,71,174];
//     let expected = 12.34;
//     assert_eq!(Term::Float(expected), decode(&input).unwrap());
// }
