pub mod bds05;
pub mod bds06;
pub mod bds08;
pub mod bds09;
pub mod bds10;
pub mod bds17;
pub mod bds20;
pub mod bds30;
pub mod bds40;
pub mod bds44;
pub mod bds50;
pub mod bds60;
pub mod bds61;
pub mod bds62;
pub mod bds65;

use serde::ser::Serializer;

fn f64_twodecimals<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let rounded_value = (value * 100.0).round() / 100.0; // Round to two decimals
    serializer.serialize_f64(rounded_value)
}

fn op_f64_threedecimals<S>(
    value: &Option<f64>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(value) = value {
        let rounded_value = (value * 1000.0).round() / 1000.0; // Round to three decimals
        serializer.serialize_f64(rounded_value)
    } else {
        serializer.serialize_none()
    }
}
