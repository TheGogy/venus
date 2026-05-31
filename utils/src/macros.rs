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
