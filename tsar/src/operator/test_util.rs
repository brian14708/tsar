pub(crate) const F32_DATA: [f32; 12] = [
    0.0,
    123.4,
    -123.4,
    1e30,
    1e-30,
    f32::NAN,
    f32::EPSILON,
    std::f32::consts::PI,
    std::f32::consts::LN_2,
    std::f32::consts::E,
    f32::INFINITY,
    f32::NEG_INFINITY,
];

pub(crate) const F64_DATA: [f64; 12] = [
    0.0,
    123.4,
    -123.4,
    1e30,
    1e-30,
    f64::NAN,
    f64::EPSILON,
    std::f64::consts::PI,
    std::f64::consts::LN_2,
    std::f64::consts::E,
    f64::INFINITY,
    f64::NEG_INFINITY,
];

pub(crate) const BF16_DATA: [half::bf16; 12] = [
    half::bf16::from_f32_const(0.0),
    half::bf16::from_f32_const(123.4),
    half::bf16::from_f32_const(-123.4),
    half::bf16::from_f32_const(1e30),
    half::bf16::from_f32_const(1e-30),
    half::bf16::NAN,
    half::bf16::EPSILON,
    half::bf16::from_f32_const(std::f32::consts::PI),
    half::bf16::from_f32_const(std::f32::consts::LN_2),
    half::bf16::from_f32_const(std::f32::consts::E),
    half::bf16::INFINITY,
    half::bf16::NEG_INFINITY,
];

#[macro_export]
macro_rules! assert_delta {
    ($x:expr, $y:expr) => {
        if $x.to_bits() == $y.to_bits() {
            // ok
        } else if $x == $x {
            assert_eq!($x, $y);
        } else {
            // nan
            assert_ne!($y, $y);
        }
    };
    ($x:expr, $y:expr, $d:expr) => {
        if $x.to_bits() == $y.to_bits() {
            // ok
        } else if $x == $x {
            assert!($x - $y <= $d || $y - $x <= $d);
        } else {
            // nan
            assert_ne!($y, $y);
        }
    };
}
