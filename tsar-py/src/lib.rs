use pyo3::prelude::*;

#[pyclass(module = "tsar.tsar")]
struct Writer {
    w: tsar::write::Writer<std::fs::File>,
}

#[pymethods]
impl Writer {
    #[new]
    fn new(dst: &str) -> PyResult<Self> {
        Ok(Self {
            w: tsar::write::Writer::new(std::fs::File::create(dst)?),
        })
    }

    pub fn write_file(&mut self, name: String, d: &[u8]) -> PyResult<()> {
        self.w.write_file(name, std::io::Cursor::new(d)).unwrap();
        Ok(())
    }

    pub fn write_blob(
        &mut self,
        ty: &str,
        name: &str,
        offset: usize,
        data: &[u8],
        dims: Vec<usize>,
        relative_error: f64,
    ) -> PyResult<()> {
        let opt = tsar::write::BlobOption { relative_error };
        let ty = match ty {
            "f32" => Some(tsar::DataType::Float32),
            "f64" => Some(tsar::DataType::Float64),
            "f16" => Some(tsar::DataType::Float16),
            "bf16" => Some(tsar::DataType::Bfloat16),
            "i8" => Some(tsar::DataType::Int8),
            "u8" => Some(tsar::DataType::Uint8),
            "i16" => Some(tsar::DataType::Int16),
            "u16" => Some(tsar::DataType::Uint16),
            "i32" => Some(tsar::DataType::Int32),
            "u32" => Some(tsar::DataType::Uint32),
            "i64" => Some(tsar::DataType::Int64),
            "u64" => Some(tsar::DataType::Uint64),
            _ => None,
        };

        match ty {
            Some(ty) => self.w.write_blob(name, offset, data, ty, &dims, opt),
            _ => self
                .w
                .write_blob(name, offset, data, tsar::DataType::Byte, &[data.len()], opt),
        }
        .unwrap();
        Ok(())
    }

    pub fn close(&mut self) -> PyResult<()> {
        self.w.finish().unwrap();
        Ok(())
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn tsar(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Writer>()?;
    Ok(())
}
