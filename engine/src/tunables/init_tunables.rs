/// Macro to initialize tunable values for SPSA tuning.
///
/// Any values initialized are constant when run with release flags,
/// but with tune flags they can be altered at runtime.
///
/// The step size for each value should be ~1/20 of the range.
/// Refer to https://github.com/AndyGrant/OpenBench/wiki/SPSA-Tuning-Workloads#spsa-hyperparameters
///
/// # Example
///
/// //  name:      type = min, max, val, step;
/// init_tunables! {
///     val_pawn:   i32 = 200, 150, 250, 5;
///     val_knight: i32 = 780, 700, 900, 10;
///     val_bishop: i32 = 820, 700, 900, 10;
///     val_rook:   i32 = 1300, 1200, 1500, 10;
///     val_queen:  i32 = 2500, 2400, 2600, 10;
/// }
///
#[macro_export]
macro_rules! init_tunables {
    ($($name:ident: $t:ty = $val:expr, $min:expr, $max:expr, $step:expr;)*) => {
        pub mod tunables {
            #[cfg(feature = "tune")]
            mod storage {
                use std::sync::atomic::AtomicI32;
                $(
                    #[allow(non_upper_case_globals)]
                    pub static $name: AtomicI32 = AtomicI32::new($val as i32);
                )*
            }

            $(
                #[cfg(not(feature = "tune"))]
                #[inline]
                pub const fn $name() -> $t {
                    $val
                }

                #[cfg(feature = "tune")]
                #[inline]
                pub fn $name() -> $t {
                    use std::sync::atomic::Ordering;
                    storage::$name.load(Ordering::Relaxed) as $t
                }
            )*

            #[cfg(feature = "tune")]
            pub fn set_tunable(tunable_name: &str, val: &str) -> Result<(), &'static str> {
                use std::sync::atomic::Ordering;
                match tunable_name {
                    $(stringify!($name) => {
                        let parsed: i32 = val.parse().map_err(|_| "Invalid value")?;
                        storage::$name.store(parsed, Ordering::Relaxed);
                        Ok(())
                    },)*
                    _ => Err("Unknown option!")
                }
            }

            #[cfg(feature = "tune")]
            pub fn spsa_output_opts() -> String {
                let mut options = String::new();
                $(
                    options.push_str(&format!(
                        "option name {} type spin default {} min {} max {}\n",
                        stringify!($name),
                        $val,
                        $min,
                        $max,
                    ));
                )*
                options
            }

            #[cfg(feature = "tune")]
            pub fn spsa_output_json() -> String {
                let mut json = String::new();
                json.push_str("{\n");
                $(
                    json.push_str(&format!(
                        " \"{}\": {{\n    \"value\": {},\n    \"min_value\": {},\n    \"max_value\": {},\n    \"step\": {}\n  }},\n",
                        stringify!($name),
                        $val,
                        $min,
                        $max,
                        $step
                    ));
                )*
                json.push_str("}\n");
                json
            }

            #[cfg(feature = "tune")]
            pub fn spsa_output_txt() -> String {
                let mut txt = String::new();
                $(
                    txt.push_str(&format!(
                        "{}, int, {}.0, {}.0, {}.0, {}.0, 0.002\n",
                        stringify!($name),
                        $val,
                        $min,
                        $max,
                        $step,
                    ));
                )*
                txt
            }
        }
    }
}

/// Macro to make things constant when they should be.
/// Otherwise, allow them to be updated.
#[macro_export]
macro_rules! maybe_const {
    ($name:ident: $ty:ty = $value:expr;) => {
        #[allow(non_upper_case_globals)]
        #[cfg(not(feature = "tune"))]
        const $name: $ty = $value;

        #[allow(non_upper_case_globals)]
        #[cfg(feature = "tune")]
        let $name: $ty = $value;
    };
}
