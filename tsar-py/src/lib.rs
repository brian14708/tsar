use pyo3::prelude::*;

#[pyclass(module = "tsar.tsar")]
struct Writer {
    w: tsar::writer::Writer<std::fs::File>,
}

#[pymethods]
impl Writer {
    #[new]
    fn new(dst: &str) -> PyResult<Self> {
        Ok(Self {
            w: tsar::writer::Writer::new(std::fs::File::create(dst)?),
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
        offset: u64,
        data: &[u8],
        dims: Vec<usize>,
        level: i32,
        relative_error: f64,
    ) -> PyResult<()> {
        let reader = std::io::Cursor::new(data);
        match ty {
            "f32" => self.w.write_blob_tensor_f32(
                name,
                offset,
                reader,
                &dims,
                tsar::writer::WriteOption {
                    level,
                    relative_error,
                },
            ),
            _ => self.w.write_blob(name, offset, reader),
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
