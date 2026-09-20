macro_rules! hashmap {
    ($( $key:expr => $value:expr ),*) => {{
        let mut map = ::std::collections::HashMap::new();
        $( map.insert($key.into(), $value.into()); )*
        map
    }};
}

pub(crate) use hashmap;

/// Emit an informational log record through `tracing` when the `cli` feature enables it.
#[cfg(feature = "cli")]
macro_rules! log_info {
    ($( $arg:tt )*) => {{
        tracing::info!($( $arg )*)
    }};
}

/// Check the arguments of `log_info` without emitting anything when `tracing` is absent.
#[cfg(not(feature = "cli"))]
macro_rules! log_info {
    ($( $arg:tt )*) => {{
        let _ = format_args!($( $arg )*);
    }};
}

pub(crate) use log_info;
