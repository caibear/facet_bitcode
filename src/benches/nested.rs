use facet::Facet;
use serde::{Deserialize, Serialize};

macro_rules! nested_struct {
    ($name:ident, $($field:ty),+) => {
        #[derive(Debug, Default, PartialEq, Facet, Serialize, Deserialize, bitcode::Encode, bitcode::Decode)]
        pub struct $name ($($field),+);
    }
}

nested_struct!(T0, T1, T1);
nested_struct!(T1, T2, T2);
nested_struct!(T2, T3, T3);
nested_struct!(T3, T4, T4);
nested_struct!(T4, T5, T5);
nested_struct!(T5, T6, T6);
nested_struct!(T6, T7, T7);
nested_struct!(T7, u8, u8);

pub fn struct_tree() -> T0 {
    Default::default()
}
