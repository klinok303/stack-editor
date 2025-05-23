#[macro_export]
macro_rules! insert_into_map {
    ($map:expr, { $($key:expr => $value:expr),* $(,)? }) => {
        $(
            $map.insert($key, $value);
        )*
    };
}
