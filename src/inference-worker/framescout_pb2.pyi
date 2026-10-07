from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class EmbeddingModel(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    UNKNOWN_MODEL: _ClassVar[EmbeddingModel]
    SIGLIP_768: _ClassVar[EmbeddingModel]
    BGE_M3_DENSE: _ClassVar[EmbeddingModel]
    BGE_M3_SPARSE: _ClassVar[EmbeddingModel]
UNKNOWN_MODEL: EmbeddingModel
SIGLIP_768: EmbeddingModel
BGE_M3_DENSE: EmbeddingModel
BGE_M3_SPARSE: EmbeddingModel

class OcrConfig(_message.Message):
    __slots__ = ("enable_ocr", "languages")
    ENABLE_OCR_FIELD_NUMBER: _ClassVar[int]
    LANGUAGES_FIELD_NUMBER: _ClassVar[int]
    enable_ocr: bool
    languages: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, enable_ocr: bool = ..., languages: _Optional[_Iterable[str]] = ...) -> None: ...

class OcrLine(_message.Message):
    __slots__ = ("text", "bbox_left", "bbox_top", "bbox_right", "bbox_bottom", "score", "lang", "timestamp_ms", "payload_json")
    TEXT_FIELD_NUMBER: _ClassVar[int]
    BBOX_LEFT_FIELD_NUMBER: _ClassVar[int]
    BBOX_TOP_FIELD_NUMBER: _ClassVar[int]
    BBOX_RIGHT_FIELD_NUMBER: _ClassVar[int]
    BBOX_BOTTOM_FIELD_NUMBER: _ClassVar[int]
    SCORE_FIELD_NUMBER: _ClassVar[int]
    LANG_FIELD_NUMBER: _ClassVar[int]
    TIMESTAMP_MS_FIELD_NUMBER: _ClassVar[int]
    PAYLOAD_JSON_FIELD_NUMBER: _ClassVar[int]
    text: str
    bbox_left: float
    bbox_top: float
    bbox_right: float
    bbox_bottom: float
    score: float
    lang: str
    timestamp_ms: int
    payload_json: str
    def __init__(self, text: _Optional[str] = ..., bbox_left: _Optional[float] = ..., bbox_top: _Optional[float] = ..., bbox_right: _Optional[float] = ..., bbox_bottom: _Optional[float] = ..., score: _Optional[float] = ..., lang: _Optional[str] = ..., timestamp_ms: _Optional[int] = ..., payload_json: _Optional[str] = ...) -> None: ...

class SingleOcrTask(_message.Message):
    __slots__ = ("file_paths", "languages", "ocr_config")
    FILE_PATHS_FIELD_NUMBER: _ClassVar[int]
    LANGUAGES_FIELD_NUMBER: _ClassVar[int]
    OCR_CONFIG_FIELD_NUMBER: _ClassVar[int]
    file_paths: _containers.RepeatedScalarFieldContainer[str]
    languages: _containers.RepeatedScalarFieldContainer[str]
    ocr_config: OcrConfig
    def __init__(self, file_paths: _Optional[_Iterable[str]] = ..., languages: _Optional[_Iterable[str]] = ..., ocr_config: _Optional[_Union[OcrConfig, _Mapping]] = ...) -> None: ...

class SingleFileTask(_message.Message):
    __slots__ = ("file_path", "ocr_config")
    FILE_PATH_FIELD_NUMBER: _ClassVar[int]
    OCR_CONFIG_FIELD_NUMBER: _ClassVar[int]
    file_path: str
    ocr_config: OcrConfig
    def __init__(self, file_path: _Optional[str] = ..., ocr_config: _Optional[_Union[OcrConfig, _Mapping]] = ...) -> None: ...

class BatchTask(_message.Message):
    __slots__ = ("file_paths", "ocr_config")
    FILE_PATHS_FIELD_NUMBER: _ClassVar[int]
    OCR_CONFIG_FIELD_NUMBER: _ClassVar[int]
    file_paths: _containers.RepeatedScalarFieldContainer[str]
    ocr_config: OcrConfig
    def __init__(self, file_paths: _Optional[_Iterable[str]] = ..., ocr_config: _Optional[_Union[OcrConfig, _Mapping]] = ...) -> None: ...

class TextEmbedding(_message.Message):
    __slots__ = ("dense", "sparse")
    class SparseEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: float
        def __init__(self, key: _Optional[str] = ..., value: _Optional[float] = ...) -> None: ...
    DENSE_FIELD_NUMBER: _ClassVar[int]
    SPARSE_FIELD_NUMBER: _ClassVar[int]
    dense: _containers.RepeatedScalarFieldContainer[float]
    sparse: _containers.ScalarMap[str, float]
    def __init__(self, dense: _Optional[_Iterable[float]] = ..., sparse: _Optional[_Mapping[str, float]] = ...) -> None: ...

class TextEntry(_message.Message):
    __slots__ = ("entry_id", "source_uri", "content", "embedding", "index_time", "metadata")
    class MetadataEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    ENTRY_ID_FIELD_NUMBER: _ClassVar[int]
    SOURCE_URI_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    EMBEDDING_FIELD_NUMBER: _ClassVar[int]
    INDEX_TIME_FIELD_NUMBER: _ClassVar[int]
    METADATA_FIELD_NUMBER: _ClassVar[int]
    entry_id: str
    source_uri: str
    content: str
    embedding: TextEmbedding
    index_time: float
    metadata: _containers.ScalarMap[str, str]
    def __init__(self, entry_id: _Optional[str] = ..., source_uri: _Optional[str] = ..., content: _Optional[str] = ..., embedding: _Optional[_Union[TextEmbedding, _Mapping]] = ..., index_time: _Optional[float] = ..., metadata: _Optional[_Mapping[str, str]] = ...) -> None: ...

class TextTask(_message.Message):
    __slots__ = ("text", "source_uri", "metadata")
    class MetadataEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    TEXT_FIELD_NUMBER: _ClassVar[int]
    SOURCE_URI_FIELD_NUMBER: _ClassVar[int]
    METADATA_FIELD_NUMBER: _ClassVar[int]
    text: str
    source_uri: str
    metadata: _containers.ScalarMap[str, str]
    def __init__(self, text: _Optional[str] = ..., source_uri: _Optional[str] = ..., metadata: _Optional[_Mapping[str, str]] = ...) -> None: ...

class EncodeRequest(_message.Message):
    __slots__ = ("task_id", "file_task", "text", "batch", "ocr_task", "text_task", "max_results", "model", "session_id", "source")
    TASK_ID_FIELD_NUMBER: _ClassVar[int]
    FILE_TASK_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    BATCH_FIELD_NUMBER: _ClassVar[int]
    OCR_TASK_FIELD_NUMBER: _ClassVar[int]
    TEXT_TASK_FIELD_NUMBER: _ClassVar[int]
    MAX_RESULTS_FIELD_NUMBER: _ClassVar[int]
    MODEL_FIELD_NUMBER: _ClassVar[int]
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    SOURCE_FIELD_NUMBER: _ClassVar[int]
    task_id: str
    file_task: SingleFileTask
    text: str
    batch: BatchTask
    ocr_task: SingleOcrTask
    text_task: TextTask
    max_results: int
    model: EmbeddingModel
    session_id: str
    source: str
    def __init__(self, task_id: _Optional[str] = ..., file_task: _Optional[_Union[SingleFileTask, _Mapping]] = ..., text: _Optional[str] = ..., batch: _Optional[_Union[BatchTask, _Mapping]] = ..., ocr_task: _Optional[_Union[SingleOcrTask, _Mapping]] = ..., text_task: _Optional[_Union[TextTask, _Mapping]] = ..., max_results: _Optional[int] = ..., model: _Optional[_Union[EmbeddingModel, str]] = ..., session_id: _Optional[str] = ..., source: _Optional[str] = ...) -> None: ...

class ErrorInfo(_message.Message):
    __slots__ = ("code", "message", "context")
    CODE_FIELD_NUMBER: _ClassVar[int]
    MESSAGE_FIELD_NUMBER: _ClassVar[int]
    CONTEXT_FIELD_NUMBER: _ClassVar[int]
    code: int
    message: str
    context: str
    def __init__(self, code: _Optional[int] = ..., message: _Optional[str] = ..., context: _Optional[str] = ...) -> None: ...

class FrameResult(_message.Message):
    __slots__ = ("timestamp", "vector", "ocr_text", "file_path", "index_time", "capture_timestamp_ms", "duration_ms", "embedding_model", "dense_vector", "sparse_weights", "ocr_lines", "ocr_engine", "ocr_model")
    class SparseWeightsEntry(_message.Message):
        __slots__ = ("key", "value")
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: float
        def __init__(self, key: _Optional[str] = ..., value: _Optional[float] = ...) -> None: ...
    TIMESTAMP_FIELD_NUMBER: _ClassVar[int]
    VECTOR_FIELD_NUMBER: _ClassVar[int]
    OCR_TEXT_FIELD_NUMBER: _ClassVar[int]
    FILE_PATH_FIELD_NUMBER: _ClassVar[int]
    INDEX_TIME_FIELD_NUMBER: _ClassVar[int]
    CAPTURE_TIMESTAMP_MS_FIELD_NUMBER: _ClassVar[int]
    DURATION_MS_FIELD_NUMBER: _ClassVar[int]
    EMBEDDING_MODEL_FIELD_NUMBER: _ClassVar[int]
    DENSE_VECTOR_FIELD_NUMBER: _ClassVar[int]
    SPARSE_WEIGHTS_FIELD_NUMBER: _ClassVar[int]
    OCR_LINES_FIELD_NUMBER: _ClassVar[int]
    OCR_ENGINE_FIELD_NUMBER: _ClassVar[int]
    OCR_MODEL_FIELD_NUMBER: _ClassVar[int]
    timestamp: float
    vector: _containers.RepeatedScalarFieldContainer[float]
    ocr_text: str
    file_path: str
    index_time: float
    capture_timestamp_ms: int
    duration_ms: int
    embedding_model: EmbeddingModel
    dense_vector: _containers.RepeatedScalarFieldContainer[float]
    sparse_weights: _containers.ScalarMap[str, float]
    ocr_lines: _containers.RepeatedCompositeFieldContainer[OcrLine]
    ocr_engine: str
    ocr_model: str
    def __init__(self, timestamp: _Optional[float] = ..., vector: _Optional[_Iterable[float]] = ..., ocr_text: _Optional[str] = ..., file_path: _Optional[str] = ..., index_time: _Optional[float] = ..., capture_timestamp_ms: _Optional[int] = ..., duration_ms: _Optional[int] = ..., embedding_model: _Optional[_Union[EmbeddingModel, str]] = ..., dense_vector: _Optional[_Iterable[float]] = ..., sparse_weights: _Optional[_Mapping[str, float]] = ..., ocr_lines: _Optional[_Iterable[_Union[OcrLine, _Mapping]]] = ..., ocr_engine: _Optional[str] = ..., ocr_model: _Optional[str] = ...) -> None: ...

class SuccessPayload(_message.Message):
    __slots__ = ("frames", "text_entries")
    FRAMES_FIELD_NUMBER: _ClassVar[int]
    TEXT_ENTRIES_FIELD_NUMBER: _ClassVar[int]
    frames: _containers.RepeatedCompositeFieldContainer[FrameResult]
    text_entries: _containers.RepeatedCompositeFieldContainer[TextEntry]
    def __init__(self, frames: _Optional[_Iterable[_Union[FrameResult, _Mapping]]] = ..., text_entries: _Optional[_Iterable[_Union[TextEntry, _Mapping]]] = ...) -> None: ...

class EncodeResponse(_message.Message):
    __slots__ = ("task_id", "success", "error")
    TASK_ID_FIELD_NUMBER: _ClassVar[int]
    SUCCESS_FIELD_NUMBER: _ClassVar[int]
    ERROR_FIELD_NUMBER: _ClassVar[int]
    task_id: str
    success: SuccessPayload
    error: ErrorInfo
    def __init__(self, task_id: _Optional[str] = ..., success: _Optional[_Union[SuccessPayload, _Mapping]] = ..., error: _Optional[_Union[ErrorInfo, _Mapping]] = ...) -> None: ...

class SearchRequest(_message.Message):
    __slots__ = ("query_text", "search_media", "search_text", "top_k", "model", "session_id")
    QUERY_TEXT_FIELD_NUMBER: _ClassVar[int]
    SEARCH_MEDIA_FIELD_NUMBER: _ClassVar[int]
    SEARCH_TEXT_FIELD_NUMBER: _ClassVar[int]
    TOP_K_FIELD_NUMBER: _ClassVar[int]
    MODEL_FIELD_NUMBER: _ClassVar[int]
    SESSION_ID_FIELD_NUMBER: _ClassVar[int]
    query_text: str
    search_media: bool
    search_text: bool
    top_k: int
    model: EmbeddingModel
    session_id: str
    def __init__(self, query_text: _Optional[str] = ..., search_media: bool = ..., search_text: bool = ..., top_k: _Optional[int] = ..., model: _Optional[_Union[EmbeddingModel, str]] = ..., session_id: _Optional[str] = ...) -> None: ...

class SearchResponse(_message.Message):
    __slots__ = ("media_hits", "text_hits")
    MEDIA_HITS_FIELD_NUMBER: _ClassVar[int]
    TEXT_HITS_FIELD_NUMBER: _ClassVar[int]
    media_hits: _containers.RepeatedCompositeFieldContainer[FrameResult]
    text_hits: _containers.RepeatedCompositeFieldContainer[TextEntry]
    def __init__(self, media_hits: _Optional[_Iterable[_Union[FrameResult, _Mapping]]] = ..., text_hits: _Optional[_Iterable[_Union[TextEntry, _Mapping]]] = ...) -> None: ...
