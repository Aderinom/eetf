use crate::{Atom, ByteList};

pub trait Convert {
    fn to_atom(self) -> Atom;
    fn to_byte_list(self) -> ByteList;
}
impl Convert for String {
    fn to_atom(self) -> Atom {
        return Atom::from(self);
    }

    fn to_byte_list(self) -> ByteList {
        return ByteList::from(self);
    }
}
impl Convert for &str {
    fn to_atom(self) -> Atom {
        return Atom::from(self);
    }

    fn to_byte_list(self) -> ByteList {
        return ByteList::from(self);
    }
}

#[cfg(test)]
mod convert_test {
    use super::*;

    #[test]
    fn str_should_convert_to_atom() {
        let a = "hello".to_atom();
        assert_eq!(Atom::from("hello"), a);
    }

    #[test]
    fn str_should_convert_to_byte_list() {
        let a = "hello".to_byte_list();
        assert_eq!(ByteList::from("hello"), a);
    }

    #[test]
    fn string_should_convert_to_atom() {
        let a = "hello".to_atom();
        assert_eq!(a.name, "hello");
    }
    #[test]
    fn string_should_convert_to_byte_list() {
        let a = "hello".to_byte_list();
        assert_eq!(ByteList::from("hello"), a);
    }
}
