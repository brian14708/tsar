#[cfg(windows)]
use zfp_sys_cc as zfp_sys;

use crate::result::{Error, Result};
use crate::DataType;

use super::Codec;

pub struct Zfp<'a> {
    dt: DataType,
    dim: usize,
    shape: &'a [usize],
    prec: f64,
}

impl<'a> Zfp<'a> {
    pub fn new(dt: DataType, dim: usize, shape: &'a [usize], prec: f64) -> Self {
        Self {
            dt,
            dim,
            shape,
            prec,
        }
    }

    unsafe fn new_field(&self, data: *mut std::ffi::c_void) -> *mut zfp_sys::zfp_field {
        let mut field_shape = [1u32; 4];
        for (i, &s) in self.shape.iter().enumerate() {
            if i < self.dim {
                field_shape[i] = s as u32;
            } else {
                field_shape[self.dim - 1] *= s as u32;
            }
        }

        let dt = match self.dt {
            DataType::Float32 => zfp_sys::zfp_type_zfp_type_float,
            DataType::Float64 => zfp_sys::zfp_type_zfp_type_double,
            DataType::Int32 => zfp_sys::zfp_type_zfp_type_int32,
            DataType::Int64 => zfp_sys::zfp_type_zfp_type_int64,
            _ => panic!("ZFP: unsupported data type"),
        };

        match self.dim {
            1 => zfp_sys::zfp_field_1d(data, dt, field_shape[0]),
            2 => zfp_sys::zfp_field_2d(data, dt, field_shape[0], field_shape[1]),
            3 => zfp_sys::zfp_field_3d(data, dt, field_shape[0], field_shape[1], field_shape[2]),
            4 => zfp_sys::zfp_field_4d(
                data,
                dt,
                field_shape[0],
                field_shape[1],
                field_shape[2],
                field_shape[3],
            ),
            _ => panic!("ZFP: unsupported dim {}", self.dim),
        }
    }
}

#[allow(clippy::useless_conversion, clippy::unnecessary_cast)] // workaround for windows
impl Codec for Zfp<'_> {
    fn encode<'a, I>(&self, data: I, out: &mut super::BufferList) -> Result<()>
    where
        I: IntoIterator<Item = &'a [u8]>,
        I::IntoIter: ExactSizeIterator,
    {
        let mut data = data.into_iter();
        assert_eq!(data.len(), 1, "ZFP: must be only one input");
        let data = data.next().unwrap();
        out.reset(1);

        let field = unsafe { self.new_field(data.as_ptr() as *mut std::ffi::c_void) };

        let zfp =
            unsafe { zfp_sys::zfp_stream_open(std::ptr::null_mut() as *mut zfp_sys::bitstream) };
        unsafe {
            zfp_sys::zfp_stream_set_accuracy(zfp, self.prec);
        }

        let bufsize = unsafe { zfp_sys::zfp_stream_maximum_size(zfp, field) };
        out[0].resize(bufsize as usize, 0);

        let stream =
            unsafe { zfp_sys::stream_open(out[0].as_mut_ptr() as *mut std::ffi::c_void, bufsize) };
        unsafe {
            zfp_sys::zfp_stream_set_bit_stream(zfp, stream);
        }

        let hdr = unsafe { zfp_sys::zfp_write_header(zfp, field, zfp_sys::ZFP_HEADER_MODE) };
        let zfpsize = unsafe { hdr + zfp_sys::zfp_compress(zfp, field) };
        out[0].truncate((hdr + zfpsize) as usize);

        unsafe {
            zfp_sys::zfp_field_free(field);
            zfp_sys::zfp_stream_close(zfp);
            zfp_sys::stream_close(stream);
        }
        if hdr == 0 || zfpsize == 0 {
            return Err(Error::ZPFUnknown);
        }

        Ok(())
    }

    fn decode<'a, I>(&self, data: I, out: &mut super::BufferList) -> Result<()>
    where
        I: IntoIterator<Item = &'a [u8]>,
        I::IntoIter: ExactSizeIterator,
    {
        let mut data = data.into_iter();
        assert_eq!(data.len(), 1, "ZFP: must be only one input");
        let data = data.next().unwrap();
        out.reset(1);

        out[0].resize(self.shape.iter().product::<usize>() * self.dt.byte_len(), 0);

        let field = unsafe { self.new_field(out[0].as_mut_ptr() as *mut std::ffi::c_void) };

        let zfp =
            unsafe { zfp_sys::zfp_stream_open(std::ptr::null_mut() as *mut zfp_sys::bitstream) };

        let stream = unsafe {
            zfp_sys::stream_open(
                data.as_ptr() as *mut std::ffi::c_void,
                data.len().try_into().unwrap(),
            )
        };
        unsafe {
            zfp_sys::zfp_stream_set_bit_stream(zfp, stream);
        }

        let hdr = unsafe { zfp_sys::zfp_read_header(zfp, field, zfp_sys::ZFP_HEADER_MODE) };

        let sz = unsafe { zfp_sys::zfp_decompress(zfp, field) };

        unsafe {
            zfp_sys::zfp_field_free(field);
            zfp_sys::zfp_stream_close(zfp);
            zfp_sys::stream_close(stream);
        }

        if hdr == 0 || sz == 0 {
            return Err(Error::ZPFUnknown);
        }
        Ok(())
    }
}
