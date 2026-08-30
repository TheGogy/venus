/// Find the maximum value of some array or some pair of numbers.
///
/// # Example
/// ```
/// let x = [1, 2, 3];
/// max!(x);    // 3
/// max!(1, 2); // 2
/// ```
#[macro_export]
macro_rules! max {
    ($arr:expr) => {{
        let mut idx = 0;
        let mut max = $arr[0];
        while idx < $arr.len() {
            max = max!(max, $arr[idx]);
            idx += 1;
        }
        max
    }};
    ($a:expr, $b:expr) => {
        if $a > $b { $a } else { $b }
    };
}

/// Find the minimum value of some array or some pair of numbers.
///
/// # Example
/// ```
/// let x = [1, 2, 3];
/// min!(x);    // 1
/// min!(1, 2); // 1
/// ```
#[macro_export]
macro_rules! min {
    ($arr:expr) => {{
        let mut idx = 0;
        let mut min = $arr[0];
        while idx < $arr.len() {
            min = min!(min, $arr[idx]);
            idx += 1;
        }
        max
    }};
    ($a:expr, $b:expr) => {
        if $a < $b { $a } else { $b }
    };
}

/// Const for loop using C syntax.
///
/// # Example
/// ```
/// cfor!(let mut sq = 0; sq < 64; sq += 1; {
///     handle_square(sq);
/// });
/// ```
#[macro_export]
macro_rules! cfor {
    ($init: stmt; $cond: expr; $step: expr; $body: block) => {
        {
            $init
            while $cond {
                $body;
                $step;
            }
        }
    }
}
