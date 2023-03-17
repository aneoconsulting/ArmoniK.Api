import logging
import sys
import json
import traceback
from logging import getLogger
from datetime import datetime
from types import TracebackType
from typing import Dict, Optional, Union, Tuple, Type


class ClefLogger:
    _worker_loggers: Dict[str, "ClefLogger"] = {}

    @classmethod
    def setup_logging(cls, minlevel: int = logging.INFO) -> None:
        logging.basicConfig(level=minlevel, stream=sys.stdout, format="%(message)s")

    @classmethod
    def getLogger(cls, name):
        if name in ClefLogger._worker_loggers:
            return ClefLogger._worker_loggers[name]
        ClefLogger._worker_loggers[name] = cls(name)
        return ClefLogger._worker_loggers[name]

    def __init__(self, name: Optional[str] = None, level: int = logging.INFO):
        self._logger = getLogger(name)
        self._logger.setLevel(level)

    def info(self, message: str, **kwargs):
        self.log(logging.INFO, message, **kwargs)

    def debug(self, message: str, **kwargs):
        self.log(logging.DEBUG, message, **kwargs)

    def warning(self, message: str, **kwargs):
        self.log(logging.WARNING, message, **kwargs)

    def error(self, message: str, **kwargs):
        self.log(logging.ERROR, message, **kwargs)

    def critical(self, message: str, **kwargs):
        self.log(logging.CRITICAL, message, **kwargs)

    def exception(self, message: str, exc_info: Optional[BaseException] = None, **kwargs):
        self.log(logging.ERROR, message, exc_info=exc_info, **kwargs)

    def log(self, level: int, message: str, exc_info: Union[BaseException, Tuple[Type[BaseException], BaseException, Optional[TracebackType]], bool, None] = None, **kwargs):
        try:
            if self._logger.isEnabledFor(level):
                payload = {
                    "@t": datetime.utcnow().isoformat(),
                    "@l": logging.getLevelName(level),
                    "@mt": message,
                }
                if exc_info:
                    if isinstance(exc_info, bool):
                        exc_info = sys.exc_info()
                    elif isinstance(exc_info, BaseException):
                        exc_info = (type(exc_info), exc_info, exc_info.__traceback__)
                    exc_info = "\n".join(traceback.format_exception(*exc_info))
                    payload["@x"] = exc_info
                for k, v in kwargs:
                    if k.startswith("@"):
                        k = "@"+k
                    payload[k] = str(v)
                self._logger.log(level, msg=json.dumps(payload))
        except Exception as e:
            print(f"Couldn't log message : {e}")

    def setLevel(self, level: int):
        self._logger.setLevel(level)
