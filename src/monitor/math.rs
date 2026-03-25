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

#[cfg(test)]
mod tests {
    use super::{normalize_range, normalize_wrapped, wrapped_value};

    #[test]
    fn normalize_range_returns_midpoint_for_zero_or_negative_span() {
        assert_eq!(normalize_range(5, 10, 10), 0.5);
        assert_eq!(normalize_range(5, 10, 0), 0.5);
    }

    #[test]
    fn normalize_range_clamps_to_the_input_bounds() {
        assert_eq!(normalize_range(-5, 0, 10), 0.0);
        assert_eq!(normalize_range(15, 0, 10), 1.0);
        assert_eq!(normalize_range(5, 0, 10), 0.5);
    }

    #[test]
    fn wrapped_value_wraps_around_half_range_boundaries() {
        assert_eq!(wrapped_value(5, 10), 5);
        assert_eq!(wrapped_value(6, 10), -4);
        assert_eq!(wrapped_value(-5, 10), -5);
        assert_eq!(wrapped_value(-6, 10), 4);
    }

    #[test]
    fn normalize_wrapped_maps_center_and_endpoints_into_unit_range() {
        assert_eq!(normalize_wrapped(0, 10), 0.5);
        assert_eq!(normalize_wrapped(5, 10), 1.0);
        assert_eq!(normalize_wrapped(-5, 10), 0.0);
        assert_eq!(normalize_wrapped(6, 10), 0.1);
    }
}
