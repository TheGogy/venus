/// Macro to initialize tunable values for SPSA tuning.
///
/// Any values initialized are constant when run with release flags,
/// but with tune flags they can be altered at runtime.
///
/// The step size for each value should be ~1/20 of the range.
/// Refer to <https://github.com/AndyGrant/OpenBench/wiki/SPSA-Tuning-Workloads#spsa-hyperparameters>
///
/// # Example
/// ```
/// //  name:      type = val, min, max, step;
/// init_tunables! {
///     val_pawn:   i32 = 200, 150, 250, 5;
///     val_knight: i32 = 780, 700, 900, 10;
///     val_bishop: i32 = 820, 700, 900, 10;
///     val_rook:   i32 = 1300, 1200, 1500, 10;
///     val_queen:  i32 = 2500, 2400, 2600, 10;
/// }
///```
#[macro_export]
macro_rules! init_tunables {
    ($($name:ident: $t:tt = $val:expr, $min:expr, $max:expr, $step:expr;)*) => {
        pub mod tunables {
            #[cfg(feature = "tune")]
            mod storage {
                $crate::init_tunables!(@storage_items $($name: $t = $val, $min, $max, $step;)*);
            }

            $crate::init_tunables!(@accessors $($name: $t = $val, $min, $max, $step;)*);

            #[cfg(feature = "tune")]
            pub fn set_tunable(tunable_name: &str, val: &str) -> Result<(), &'static str> {
                $crate::init_tunables!(@set_tunable_if tunable_name, val; $($name: $t = $val, $min, $max, $step;)*)
            }

            #[cfg(feature = "tune")]
            pub fn spsa_output_opts() -> String {
                let mut options = String::new();
                $crate::init_tunables!(@spsa_output_opts_items options; $($name: $t = $val, $min, $max, $step;)*);
                options
            }

            #[cfg(feature = "tune")]
            pub fn spsa_output_txt() -> String {
                let mut txt = String::new();
                $crate::init_tunables!(@spsa_output_txt_items txt; $($name: $t = $val, $min, $max, $step;)*);
                txt
            }
        }
    };

    (@storage_items) => {};
    (@storage_items $name:ident: f32 = $val:expr, $min:expr, $max:expr, $step:expr; $($rest:tt)*) => {
        #[allow(non_upper_case_globals)]
        pub static $name: std::sync::atomic::AtomicU32 =
            std::sync::atomic::AtomicU32::new(($val as f32).to_bits());
        $crate::init_tunables!(@storage_items $($rest)*);
    };
    (@storage_items $name:ident: $t:ty = $val:expr, $min:expr, $max:expr, $step:expr; $($rest:tt)*) => {
        #[allow(non_upper_case_globals)]
        pub static $name: std::sync::atomic::AtomicI32 =
            std::sync::atomic::AtomicI32::new($val as i32);
        $crate::init_tunables!(@storage_items $($rest)*);
    };

    (@accessors) => {};
    (@accessors $name:ident: f32 = $val:expr, $min:expr, $max:expr, $step:expr; $($rest:tt)*) => {
        #[cfg(not(feature = "tune"))]
        pub const fn $name() -> f32 {
            $val
        }

        #[cfg(feature = "tune")]
        pub fn $name() -> f32 {
            use std::sync::atomic::Ordering;
            f32::from_bits(storage::$name.load(Ordering::Relaxed))
        }

        $crate::init_tunables!(@accessors $($rest)*);
    };
    (@accessors $name:ident: $t:ty = $val:expr, $min:expr, $max:expr, $step:expr; $($rest:tt)*) => {
        #[cfg(not(feature = "tune"))]
        pub const fn $name() -> $t {
            $val
        }

        #[cfg(feature = "tune")]
        pub fn $name() -> $t {
            use std::sync::atomic::Ordering;
            storage::$name.load(Ordering::Relaxed) as $t
        }

        $crate::init_tunables!(@accessors $($rest)*);
    };

    (@set_tunable_if $name_expr:expr, $val_expr:expr;) => {
        Err("Unknown option!")
    };
    (@set_tunable_if $name_expr:expr, $val_expr:expr; $name:ident: f32 = $val:expr, $min:expr, $max:expr, $step:expr; $($rest:tt)*) => {
        if $name_expr == stringify!($name) {
            let parsed: f32 = $val_expr.parse().map_err(|_| "Invalid value")?;
            storage::$name.store(parsed.to_bits(), std::sync::atomic::Ordering::Relaxed);
            Ok(())
        } else {
            $crate::init_tunables!(@set_tunable_if $name_expr, $val_expr; $($rest)*)
        }
    };
    (@set_tunable_if $name_expr:expr, $val_expr:expr; $name:ident: $t:ty = $val:expr, $min:expr, $max:expr, $step:expr; $($rest:tt)*) => {
        if $name_expr == stringify!($name) {
            let parsed: i32 = $val_expr.parse().map_err(|_| "Invalid value")?;
            storage::$name.store(parsed, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        } else {
            $crate::init_tunables!(@set_tunable_if $name_expr, $val_expr; $($rest)*)
        }
    };

    (@spsa_output_opts_items $options:ident;) => {};
    (@spsa_output_opts_items $options:ident; $name:ident: f32 = $val:expr, $min:expr, $max:expr, $step:expr; $($rest:tt)*) => {
        $options.push_str(&format!(
            "option name {} type string default {}\n",
            stringify!($name),
            $val,
        ));
        $crate::init_tunables!(@spsa_output_opts_items $options; $($rest)*);
    };
    (@spsa_output_opts_items $options:ident; $name:ident: $t:ty = $val:expr, $min:expr, $max:expr, $step:expr; $($rest:tt)*) => {
        $options.push_str(&format!(
            "option name {} type spin default {} min {} max {}\n",
            stringify!($name),
            $val,
            $min,
            $max,
        ));
        $crate::init_tunables!(@spsa_output_opts_items $options; $($rest)*);
    };

    (@spsa_output_txt_items $txt:ident;) => {};
    (@spsa_output_txt_items $txt:ident; $name:ident: f32 = $val:expr, $min:expr, $max:expr, $step:expr; $($rest:tt)*) => {
        $txt.push_str(&format!(
            "{}, float, {}, {}, {}, {}, 0.002\n",
            stringify!($name),
            $val,
            $min,
            $max,
            $step,
        ));
        $crate::init_tunables!(@spsa_output_txt_items $txt; $($rest)*);
    };
    (@spsa_output_txt_items $txt:ident; $name:ident: $t:ty = $val:expr, $min:expr, $max:expr, $step:expr; $($rest:tt)*) => {
        $txt.push_str(&format!(
            "{}, int, {}.0, {}.0, {}.0, {}.0, 0.002\n",
            stringify!($name),
            $val,
            $min,
            $max,
            $step,
        ));
        $crate::init_tunables!(@spsa_output_txt_items $txt; $($rest)*);
    };
}
