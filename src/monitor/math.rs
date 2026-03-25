pub fn normalize_range(value: i32, min: i32, max: i32) -> f64 {
    let range = (max - min) as f64;
    if range > 0.0 {
        ((value - min) as f64 / range).clamp(0.0, 1.0)
    } else {
        0.5
    }
}

pub fn wrapped_value(value: i32, range: i32) -> i32 {
    let half_range = range / 2;
    let mut wrapped = value % range;
    if wrapped > half_range {
        wrapped -= range;
    } else if wrapped < -half_range {
        wrapped += range;
    }
    wrapped
}

pub fn normalize_wrapped(value: i32, range: i32) -> f64 {
    let half_range = range / 2;
    let wrapped = wrapped_value(value, range);
    ((wrapped + half_range) as f64 / range as f64).clamp(0.0, 1.0)
}
