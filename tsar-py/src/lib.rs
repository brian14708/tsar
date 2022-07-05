use pyo3::prelude::*;

/// Formats the sum of two numbers as string.
#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
    Ok((a + b).to_string())
}

#[pyfunction]
fn compress_f32(buf: &[u8], dst: String, level: i32, _error: f64) -> PyResult<()> {
    let compressor = tsar::Compressor::new(if level <= 0 {
        vec![
            tsar::Stage::ColumnarSplit(tsar::ColumnarSplitMode::Float32),
            tsar::Stage::Compress(tsar::CompressMode::Zstd(0)),
        ]
    } else if level == 1 {
        vec![
            tsar::Stage::DataConvert(tsar::DataConvertMode::Float32ToBfloat16),
            tsar::Stage::ColumnarSplit(tsar::ColumnarSplitMode::Bfloat16),
            tsar::Stage::Compress(tsar::CompressMode::Zstd(0)),
        ]
    } else {
        vec![
            tsar::Stage::DeltaEncode(tsar::DeltaEncodeMode::DiffDiffFloat32),
            tsar::Stage::DataConvert(tsar::DataConvertMode::Float32ToBfloat16),
            tsar::Stage::ColumnarSplit(tsar::ColumnarSplitMode::Bfloat16),
            tsar::Stage::Compress(tsar::CompressMode::Zstd(0)),
        ]
    });

    use std::io::Cursor;
    let mut cur = Cursor::new(buf);
    let mut i = 0;
    compressor
        .compress(&mut cur, || {
            i += 1;
            Ok(Box::new(std::fs::File::create(format!("{}.{}", dst, i))?))
        })
        .unwrap();
    Ok(())
}

/// A Python module implemented in Rust.
#[pymodule]
fn tsar(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    m.add_function(wrap_pyfunction!(compress_f32, m)?)?;
    Ok(())
}
