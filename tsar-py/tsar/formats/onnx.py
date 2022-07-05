import pathlib
import sys
import itertools
import array
import re
import json
from typing import Callable, Iterable
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


# pylint: disable=too-many-arguments,too-many-locals
def save(
    name: str,
    src: pathlib.Path,
    dst: _tsar.Writer,
    level: int,
    error: float,
    size_limit: int = 16 * 1024,
):
    model = onnx.load(str(src))
    tensors = _get_all_tensors(model)
    blob_list = []
    for idx, tensor in enumerate(tensors):
        tensor_location = tensor.name
        if not re.match('^[^<>:;,?"*|/]+$', tensor_location):
            tensor_location = f"__tensor_{idx}"
        tensor_location = str(pathlib.Path("data") / name / tensor_location)
        save_external = False

        if tensor.data_type == onnx.TensorProto.FLOAT:
            if (
                tensor.HasField("raw_data")
                and sys.getsizeof(tensor.raw_data) >= size_limit
            ):
                dst.write_blob_f32(
                    tensor_location, tensor.raw_data, list(tensor.dims), level, error
                )
                tensor.ClearField("raw_data")
                save_external = True
            elif len(tensor.float_data) * 4 >= size_limit:
                data = array.array("f")
                for val in tensor.float_data:
                    data.append(val)
                dst.write_blob_f32(
                    tensor_location, data.tobytes(), list(tensor.dims), level, error
                )
                tensor.ClearField("float_data")
                save_external = True

        if save_external:
            blob_list.append(tensor_location)
            tensor.name = tensor_location
            tensor.data_location = onnx.TensorProto.EXTERNAL
            tensor.ClearField("external_data")
            entry = tensor.external_data.add()
            entry.key = "location"
            entry.value = tensor_location

    dst.write_file(
        f".{name}.json",
        json.dumps(
            {
                "blobs": blob_list,
            }
        ).encode(),
    )
    dst.write_file(name, model.SerializeToString())
