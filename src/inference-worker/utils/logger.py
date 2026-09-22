"""
日志初始化：将 stdout / stderr 重定向到临时目录下的日志文件。
"""
import os
import sys
import datetime

from config import get_log_dir


def init_logger():
    """
    创建日志文件并重定向标准输出/错误。
    返回日志文件的完整路径。
    """
    log_dir = get_log_dir()
    log_filename = f"ai_worker_{datetime.datetime.now().strftime('%Y%m%d_%H%M%S')}.log"
    log_path = os.path.join(log_dir, log_filename)

    sys.stdout = open(log_path, "w", encoding="utf-8", buffering=1)
    sys.stderr = sys.stdout
    return log_path