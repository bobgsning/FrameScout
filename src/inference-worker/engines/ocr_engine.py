"""
OCR 引擎：EasyOCR 懒加载 + 多语言缓存。
"""
import easyocr
import numpy as np
from config import DEFAULT_OCR_LANGS


class OCRManager:
    """
    根据请求的语言组合动态创建和管理 EasyOCR Reader 实例。
    避免为不使用的语言预分配模型内存。
    """

    def __init__(self, model_storage_dir: str):
        """
        Args:
            model_storage_dir: EasyOCR 模型存储目录
        """
        self.model_storage_dir = model_storage_dir
        self._readers = {}

    def get_reader(self, languages: list[str]) -> easyocr.Reader:
        """获取（或懒加载）指定语言的 EasyOCR Reader。"""
        if not languages:
            languages = DEFAULT_OCR_LANGS

        cache_key = "_".join(sorted(languages))
        if cache_key not in self._readers:
            print(f"⏳ [OCR] Dynamic loading EasyOCR model for languages: {languages}")
            try:
                self._readers[cache_key] = easyocr.Reader(
                    languages,
                    gpu=True,
                    model_storage_directory=self.model_storage_dir,
                    download_enabled=False
                )
            except Exception as e:
                print(f"⚠️ [OCR] GPU init failed, falling back to CPU for {languages}: {e}")
                self._readers[cache_key] = easyocr.Reader(
                    languages,
                    gpu=False,
                    model_storage_directory=self.model_storage_dir,
                    download_enabled=False
                )
        return self._readers[cache_key]

    def process_image(self, img_input: np.ndarray | str, languages: list[str]) -> str:
        """
        对一张图片执行 OCR。
        Args:
            img_input: 可直接传给 EasyOCR 的图像输入（numpy RGB 数组或路径字符串）
            languages: 语言列表，如 ['en', 'ch_sim']
        Returns:
            识别出的文本字符串（空格连接）
        """
        try:
            reader = self.get_reader(languages)
            # detail=0 时 EasyOCR 契约上返回 list[str]；但其存根类型是一个宽松联合
            #（可能含 dict / list / Unknown）。这里按 str 收窄，既适配 str.join，
            # 也顺带丢弃任何非字符串异常项，保证 join 永不抛 TypeError。
            res = reader.readtext(img_input, detail=0)
            return " ".join(item for item in res if isinstance(item, str))
        except Exception as e:
            print(f"⚠️ OCR extraction failed: {e}")
            return ""