#[cfg(not(feature = "logging"))]
#[doc(hidden)]
#[macro_export]
macro_rules! debug {
    ($($element:expr), *) => {
        #[cfg(feature = "logging-print")]
        println!($($element, )*);
    };
}
