import pathlib
import tempfile
import os
import itertools
from typing import Callable, Dict, Iterable, List, Tuple
import onnx


def _get_all_tensors(onnx_model_proto: onnx.ModelProto) -> Iterable[onnx.TensorProto]:
    """Scan an ONNX model for all tensors and return as an iterator."""
    return itertools.chain(
        _get_initializer_tensors(onnx_model_proto),
        _get_attribute_tensors(onnx_model_proto),
    )


def _recursive_attribute_processor(
    attribute: onnx.AttributeProto,
    func: Callable[[onnx.GraphProto], Iterable[onnx.TensorProto]],
) -> Iterable[onnx.TensorProto]:
    """Create an iterator through processing ONNX model attributes with functor."""
    if attribute.type == onnx.AttributeProto.AttributeType.GRAPH:
        yield from func(attribute.g)
    if attribute.type == onnx.AttributeProto.AttributeType.GRAPHS:
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


def _get_initializer_tensors(
    onnx_model_proto: onnx.ModelProto,
) -> Iterable[onnx.TensorProto]:
    """Create an iterator of initializer tensors from ONNX model."""
    yield from _get_initializer_tensors_from_graph(onnx_model_proto.graph)


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


def _get_attribute_tensors(
    onnx_model_proto: onnx.ModelProto,
) -> Iterable[onnx.TensorProto]:
    """Create an iterator of tensors from node attributes of an ONNX model."""
    yield from _get_attribute_tensors_from_graph(onnx_model_proto.graph)


def save(name: str, src: pathlib.Path, dst: pathlib.Path):
    model = onnx.load(src)
    with tempfile.TemporaryDirectory() as tmp:
        onnx.save(
            model,
            os.path.join(tmp, name),
            location="_" + name + ".data",
            save_as_external_data=True,
        )
        blks: Dict[str, List[Tuple[int, int, onnx.TensorProto]]] = {}
        for i in _get_all_tensors(model):
            if len(i.external_data) > 0:
                external_data = {e.key: e.value for e in i.external_data}
                blks.setdefault(external_data["location"], [])
                blks[external_data["location"]].append(
                    (int(external_data["offset"]), int(external_data["length"]), i)
                )
        print(blks)
        print(os.listdir(tmp))
        print(tmp)
    print(src, dst)
