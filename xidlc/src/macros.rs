macro_rules! hashmap {
    ($( $key:expr => $value:expr ),*) => {{
        let mut map = ::std::collections::HashMap::new();
        $( map.insert($key.into(), $value.into()); )*
        map
    }};
}

pub(crate) use hashmap;
