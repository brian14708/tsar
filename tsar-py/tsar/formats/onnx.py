import pathlib
import sys
import itertools
import array
import json
from typing import Callable, Iterable, Optional
import onnx

import tsar.tsar as _tsar


def _get_all_tensors(onnx_model_proto: onnx.ModelProto) -> Iterable[onnx.TensorProto]:
    """Scan an ONNX model for all tensors and return as an iterator."""
    return itertools.chain(
        _get_initializer_tensors_from_graph(onnx_model_proto.graph),
        _get_attribute_tensors_from_graph(onnx_model_proto.graph),
    )


def _recursive_attribute_processor(
    attribute: onnx.AttributeProto,
    func: Callable[[onnx.GraphProto], Iterable[onnx.TensorProto]],
) -> Iterable[onnx.TensorProto]:
    """Create an iterator through processing ONNX model attributes with functor."""
    if attribute.type == onnx.AttributeProto.AttributeType.GRAPH:
        yield from func(attribute.g)
    elif attribute.type == onnx.AttributeProto.AttributeType.GRAPHS:
        for graph in attribute.graphs:
            yield from func(graph)


def _get_initializer_tensors_from_graph(
    onnx_model_proto_graph: onnx.GraphProto,
) -> Iterable[onnx.TensorProto]:
    """Create an iterator of initializer tensors from ONNX model graph."""
    yield from onnx_model_proto_graph.initializer
    for node in onnx_model_proto_graph.node:
        for attribute in node.attribute:
            yield from _recursive_attribute_processor(
                attribute, _get_initializer_tensors_from_graph
            )


def _get_attribute_tensors_from_graph(
    onnx_model_proto_graph: onnx.GraphProto,
) -> Iterable[onnx.TensorProto]:
    """Create an iterator of tensors from node attributes of an ONNX model graph."""
    for node in onnx_model_proto_graph.node:
        for attribute in node.attribute:
            if attribute.HasField("t"):
                yield attribute.t
            yield from attribute.tensors
            yield from _recursive_attribute_processor(
                attribute, _get_attribute_tensors_from_graph
            )


def _num_elements(tensor: onnx.TensorProto) -> int:
    """Return the number of elements in a tensor."""
    p = 1
    for dim in tensor.dims:
        p *= dim
    return p


# pylint: disable=too-many-arguments,too-many-locals,too-many-branches,too-many-statements
def save(
    name: str,
    src: pathlib.Path,
    dst: _tsar.Writer,
    error: float,
    size_limit: int = 16 * 1024,
    progress_fn: Optional[Callable[[int, int], None]] = None,
):
    model = onnx.load(str(src))
    tensors = sorted(_get_all_tensors(model), key=_num_elements)
    blob_list = {}
    tensor_location = str(pathlib.Path(name).with_suffix(".data"))
    tensor_offset = 0
    for idx, tensor in enumerate(tensors):
        if progress_fn:
            progress_fn(idx, len(tensors))
        if not tensor.name:
            continue
        save_external = None

        if tensor.data_type == onnx.TensorProto.FLOAT:
            if (
                tensor.HasField("raw_data")
                and sys.getsizeof(tensor.raw_data) >= size_limit
            ):
                save_external = ("f32", tensor.raw_data)
                tensor.ClearField("raw_data")
            elif len(tensor.float_data) * 4 >= size_limit:
                data_f32 = array.array("f")
                for val in tensor.float_data:
                    data_f32.append(val)
                save_external = ("f32", data_f32.tobytes())
                tensor.ClearField("float_data")

        elif tensor.data_type == onnx.TensorProto.DOUBLE:
            if (
                tensor.HasField("raw_data")
                and sys.getsizeof(tensor.raw_data) >= size_limit
            ):
                save_external = ("f64", tensor.raw_data)
                tensor.ClearField("raw_data")
            elif len(tensor.double_data) * 8 >= size_limit:
                data_f64 = array.array("d")
                for val in tensor.double_data:
                    data_f64.append(val)
                save_external = ("f64", data_f64.tobytes())
                tensor.ClearField("double_data")

        elif tensor.data_type == onnx.TensorProto.BFLOAT16:
            if (
                tensor.HasField("raw_data")
                and sys.getsizeof(tensor.raw_data) >= size_limit
            ):
                save_external = ("bf16", tensor.raw_data)
                tensor.ClearField("raw_data")

        elif tensor.data_type == onnx.TensorProto.FLOAT16:
            if (
                tensor.HasField("raw_data")
                and sys.getsizeof(tensor.raw_data) >= size_limit
            ):
                save_external = ("f16", tensor.raw_data)
                tensor.ClearField("raw_data")
            elif len(tensor.int32_data) * 2 >= size_limit:
                data_f16 = array.array("H")
                for val in tensor.int32_data:
                    data_f16.append(val)
                save_external = ("f16", data_f16.tobytes())
                tensor.ClearField("int32_data")

        elif tensor.data_type == onnx.TensorProto.INT8:
            if (
                tensor.HasField("raw_data")
                and sys.getsizeof(tensor.raw_data) >= size_limit
            ):
                save_external = ("i8", tensor.raw_data)
                tensor.ClearField("raw_data")
            elif len(tensor.int32_data) * 1 >= size_limit:
                data_i8 = array.array("b")
                for val in tensor.int32_data:
                    data_i8.append(val)
                save_external = ("i8", data_i8.tobytes())
                tensor.ClearField("int32_data")

        elif tensor.data_type == onnx.TensorProto.UINT8:
            if (
                tensor.HasField("raw_data")
                and sys.getsizeof(tensor.raw_data) >= size_limit
            ):
                save_external = ("u8", tensor.raw_data)
                tensor.ClearField("raw_data")
            elif len(tensor.int32_data) * 1 >= size_limit:
                data_u8 = array.array("B")
                for val in tensor.int32_data:
                    data_u8.append(val)
                save_external = ("u8", data_u8.tobytes())
                tensor.ClearField("int32_data")

        elif tensor.data_type == onnx.TensorProto.INT16:
            if (
                tensor.HasField("raw_data")
                and sys.getsizeof(tensor.raw_data) >= size_limit
            ):
                save_external = ("i16", tensor.raw_data)
                tensor.ClearField("raw_data")
            elif len(tensor.int32_data) * 2 >= size_limit:
                data_i16 = array.array("h")
                for val in tensor.int32_data:
                    data_i16.append(val)
                save_external = ("i16", data_i16.tobytes())
                tensor.ClearField("int32_data")

        elif tensor.data_type == onnx.TensorProto.UINT16:
            if (
                tensor.HasField("raw_data")
                and sys.getsizeof(tensor.raw_data) >= size_limit
            ):
                save_external = ("u16", tensor.raw_data)
                tensor.ClearField("raw_data")
            elif len(tensor.int32_data) * 2 >= size_limit:
                data_u16 = array.array("H")
                for val in tensor.int32_data:
                    data_u16.append(val)
                save_external = ("u16", data_u16.tobytes())
                tensor.ClearField("int32_data")

        elif tensor.data_type == onnx.TensorProto.INT32:
            if (
                tensor.HasField("raw_data")
                and sys.getsizeof(tensor.raw_data) >= size_limit
            ):
                save_external = ("i32", tensor.raw_data)
                tensor.ClearField("raw_data")
            elif len(tensor.int32_data) * 4 >= size_limit:
                data_i32 = array.array("l")
                for val in tensor.int32_data:
                    data_i32.append(val)
                save_external = ("i32", data_i32.tobytes())
                tensor.ClearField("int32_data")

        elif tensor.data_type == onnx.TensorProto.UINT32:
            if (
                tensor.HasField("raw_data")
                and sys.getsizeof(tensor.raw_data) >= size_limit
            ):
                save_external = ("u32", tensor.raw_data)
                tensor.ClearField("raw_data")
            elif len(tensor.uint64_data) * 4 >= size_limit:
                data_u32 = array.array("L")
                for val in tensor.uint64_data:
                    data_u32.append(val)
                save_external = ("u32", data_u32.tobytes())
                tensor.ClearField("uint64_data")

        elif tensor.data_type == onnx.TensorProto.INT64:
            if (
                tensor.HasField("raw_data")
                and sys.getsizeof(tensor.raw_data) >= size_limit
            ):
                save_external = ("i64", tensor.raw_data)
                tensor.ClearField("raw_data")
            elif len(tensor.int64_data) * 8 >= size_limit:
                data_i64 = array.array("q")
                for val in tensor.int64_data:
                    data_i64.append(val)
                save_external = ("i64", data_i64.tobytes())
                tensor.ClearField("int64_data")

        elif tensor.data_type == onnx.TensorProto.UINT64:
            if (
                tensor.HasField("raw_data")
                and sys.getsizeof(tensor.raw_data) >= size_limit
            ):
                save_external = ("u64", tensor.raw_data)
                tensor.ClearField("raw_data")
            elif len(tensor.uint64_data) * 8 >= size_limit:
                data_u64 = array.array("Q")
                for val in tensor.uint64_data:
                    data_u64.append(val)
                save_external = ("u64", data_u64.tobytes())
                tensor.ClearField("uint64_data")

        else:
            # unknown data type
            if (
                tensor.HasField("raw_data")
                and sys.getsizeof(tensor.raw_data) >= size_limit
            ):
                save_external = ("", tensor.raw_data)
                tensor.ClearField("raw_data")

        if save_external:
            blob_name = f"{name}[{idx}]"
            dst.write_blob(
                save_external[0],
                blob_name,
                save_external[1],
                list(tensor.dims),
                error,
                (tensor_location, tensor_offset),
            )
            blob_list[tensor.name] = blob_name
            tensor.data_location = onnx.TensorProto.EXTERNAL
            tensor.ClearField("external_data")
            for (k, v) in {
                "location": tensor_location,
                "offset": tensor_offset,
                "length": len(save_external[1]),
            }.items():
                entry = tensor.external_data.add()
                entry.key = k
                entry.value = str(v)
            tensor_offset += len(save_external[1])

    if progress_fn:
        progress_fn(len(tensors), len(tensors))
    dst.write_file(
        f".{name}.json",
        json.dumps(
            {
                "blobs": blob_list,
            }
        ).encode(),
    )
    dst.write_file(name, model.SerializeToString())
