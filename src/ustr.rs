use arrayvec::ArrayString;
use std::collections::HashMap;

pub type Ustr = ArrayString<32>;
pub type UstrMap<V> = HashMap<Ustr, V>;
pub fn ustr(s: &str) -> Ustr {
    Ustr::from(s).unwrap()
}
