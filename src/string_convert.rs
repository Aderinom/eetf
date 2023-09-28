use crate::{Atom, ByteList};

trait Convert {
    fn as_atom(self) -> Atom;
    fn as_byte_list(self) -> ByteList;
}
impl Convert for String {
    fn as_atom(self) -> Atom {
        return Atom::from(self);
    }

    fn as_byte_list(self) -> ByteList {
        return ByteList::from(self);
    }
}
impl Convert for &str {
    fn as_atom(self) -> Atom {
        return Atom::from(self);
    }

    fn as_byte_list(self) -> ByteList {
        return ByteList::from(self);
    }
}

#[cfg(test)]
mod convert_test {
    use super::*;

    #[test]
    fn str_should_convert_to_atom() {
        let a = "hello".as_atom();
        assert_eq!(Atom::from("hello"), a);
    }

    #[test]
    fn str_should_convert_to_byte_list() {
        let a = "hello".as_byte_list();
        assert_eq!(ByteList::from("hello"), a);
    }

    #[test]
    fn string_should_convert_to_atom() {
        let a = "hello".as_atom();
        assert_eq!(a.name, "hello");
    }
    #[test]
    fn string_should_convert_to_byte_list() {
        let a = "hello".as_byte_list();
        assert_eq!(ByteList::from("hello"), a);
    }
}
