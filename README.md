# Tensor Archive

Archive file format for storing tensors, with optional lossy compression for better storage efficiency.

Features:
- floating-point compression ([zfp](https://github.com/LLNL/zfp))
- storing data in lower precision format (bfloat16, ...)
- compressing mantissa and exponents separately
- tools for building archives from ONNX format

## Quick Start

Install `tsar-py` package from source (require Rust build environment):

```sh
pip install git+https://github.com/brian14708/tsar.git#subdirectory=tsar-py
```

### ONNX format

```sh
# lossless compression
tsar-pack -e 0 "<ONNX-FILE>.onnx" output.tsar
# lossy compression (with maximum error 1e-6)
tsar-pack -e 1e-6 "<ONNX-FILE>.onnx" output.tsar

# extract file to model/ directory
tsar-unpack output.tsar model/
```

### Python API

TODO

## Results

| Model                                                                               | Compression     | Size      |
| ----------------------------------------------------------------------------------- | --------------- | --------- |
| [ResNet-152](https://github.com/onnx/models/tree/main/vision/classification/resnet) | none            | 230.6 MiB |
|                                                                                     | gzip            | 215.4 MiB |
|                                                                                     | tsar (lossless) | 197.4 MiB |
|                                                                                     | tsar (err=1e-6) | 129.8 MiB |
|                                                                                     | tsar (err=1e-5) | 108.7 MiB |
|                                                                                     | tsar (err=1e-4) | 87.8 MiB  |
|                                                                                     | tsar (err=1e-3) | 60.6 MiB  |


