#[cfg(not(feature = "logging"))]
#[doc(hidden)]
#[macro_export]
macro_rules! debug {
    ($($element:expr), *) => {
        #[cfg(feature = "logging-print")]
        println!($($element, )*);
    };
}

#[cfg(not(feature = "logging"))]
#[doc(hidden)]
#[macro_export]
macro_rules! info {
    ($($element:expr), *) => {
        #[cfg(feature = "logging-print")]
        println!($($element, )*);
    };
}

#[cfg(not(feature = "logging"))]
#[doc(hidden)]
#[macro_export]
macro_rules! warn {
    ($($element:expr), *) => {
        #[cfg(feature = "logging-print")]
        println!($($element, )*);
    };
}

#[cfg(not(feature = "logging"))]
#[doc(hidden)]
#[macro_export]
macro_rules! error {
    ($($element:expr), *) => {
        #[cfg(feature = "logging-print")]
        println!($($element, )*);
    };
}
