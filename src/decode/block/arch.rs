//! Conditionally compile one (and only one) of the files from within the arch
//! directory.
use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "simd")] {
        cfg_if! {
            if #[cfg(any(target_arch = "x86", target_arch = "x86_64"))] {
                pub(crate) mod x86;
            } else {
                pub(crate) mod other;
            }
        }
    } else {
        pub(crate) mod other;
    }
}
