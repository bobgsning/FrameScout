"""
search_pb2 的类型桩（手写，与 src/proto/search.proto v3.0.0 一一对应）。

背景：protobuf 4.25+ 的生成代码通过 _builder 动态注入消息类，
search_pb2.py 里没有静态 class 定义，类型检查器（basedpyright /
pyright / mypy）因此无法识别 EncodeRequest、FrameResult 等符号，
产生大量 "不是模块的已知属性" 误报。本桩文件放在 search_pb2.py
旁边即可生效，同时为 IDE 提供完整补全。

⚠️ 维护约定：修改 search.proto 并重新生成 search_pb2.py 后，
必须同步更新本文件。
"""

from typing import Iterable, Literal, Optional

from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from google.protobuf.internal import containers as _containers

DESCRIPTOR: _descriptor.FileDescriptor


class OcrConfig(_message.Message):
    enable_ocr: bool
    languages: _containers.RepeatedScalarFieldContainer[str]

    def __init__(
        self,
        *,
        enable_ocr: bool = ...,
        languages: Optional[Iterable[str]] = ...,
    ) -> None: ...


class SingleOcrTask(_message.Message):
    file_paths: _containers.RepeatedScalarFieldContainer[str]
    languages: _containers.RepeatedScalarFieldContainer[str]

    def __init__(
        self,
        *,
        file_paths: Optional[Iterable[str]] = ...,
        languages: Optional[Iterable[str]] = ...,
    ) -> None: ...


class BatchPaths(_message.Message):
    file_paths: _containers.RepeatedScalarFieldContainer[str]
    ocr_config: OcrConfig

    def __init__(
        self,
        *,
        file_paths: Optional[Iterable[str]] = ...,
        ocr_config: Optional[OcrConfig] = ...,
    ) -> None: ...


class EncodeRequest(_message.Message):
    task_id: str
    # 以下均属 oneof payload：未设置时访问返回字段默认值
    file_path: str
    text: str
    batch: BatchPaths
    ocr_task: SingleOcrTask
    # oneof 外的独立 message 字段（field 6），可用 HasField 判断是否设置
    single_file_ocr_config: OcrConfig

    def __init__(
        self,
        *,
        task_id: str = ...,
        file_path: str = ...,
        text: str = ...,
        batch: Optional[BatchPaths] = ...,
        ocr_task: Optional[SingleOcrTask] = ...,
        single_file_ocr_config: Optional[OcrConfig] = ...,
    ) -> None: ...

    def WhichOneof(
        self, oneof_group: Literal["payload"]
    ) -> Optional[Literal["file_path", "text", "batch", "ocr_task"]]: ...

    def HasField(
        self,
        field_name: Literal["batch", "ocr_task", "single_file_ocr_config", "payload"],
    ) -> bool: ...


class ErrorInfo(_message.Message):
    code: int
    message: str
    context: str

    def __init__(
        self,
        *,
        code: int = ...,
        message: str = ...,
        context: str = ...,
    ) -> None: ...


class FrameResult(_message.Message):
    timestamp: float
    vector: _containers.RepeatedScalarFieldContainer[float]
    ocr_text: str
    file_path: str
    index_time: float

    def __init__(
        self,
        *,
        timestamp: float = ...,
        vector: Optional[Iterable[float]] = ...,
        ocr_text: str = ...,
        file_path: str = ...,
        index_time: float = ...,
    ) -> None: ...


class SuccessPayload(_message.Message):
    frames: _containers.RepeatedCompositeFieldContainer[FrameResult]

    def __init__(
        self,
        *,
        frames: Optional[Iterable[FrameResult]] = ...,
    ) -> None: ...


class EncodeResponse(_message.Message):
    task_id: str
    # oneof result
    success: SuccessPayload
    error: ErrorInfo

    def __init__(
        self,
        *,
        task_id: str = ...,
        success: Optional[SuccessPayload] = ...,
        error: Optional[ErrorInfo] = ...,
    ) -> None: ...

    def WhichOneof(
        self, oneof_group: Literal["result"]
    ) -> Optional[Literal["success", "error"]]: ...
