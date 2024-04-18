from __future__ import annotations
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
        """Activates logging, should only be done once per process

        Args:
            minlevel: Minimum level of logging for the whole process
        """
        logging.basicConfig(level=minlevel, stream=sys.stdout, format="%(message)s")

    @classmethod
    def getLogger(cls, name: str) -> "ClefLogger":
        """Get the logger with the given name. Creates it if it doesn't exist

        Args:
            name: Name of the logger

        Returns:
            ClefLogger logger of the requested name
        """
        if name in ClefLogger._worker_loggers:
            return ClefLogger._worker_loggers[name]
        ClefLogger._worker_loggers[name] = cls(name)
        return ClefLogger._worker_loggers[name]

    def __init__(self, name: Optional[str] = None, level: int = logging.INFO):
        self._logger = getLogger(name)
        self._logger.setLevel(level)

    def info(self, message: str, **kwargs):
        """Log a message at the info level

        Args:
            message: Message content, can contain '{name}' where name is a keyword argument given to this function (see kwargs)
            **kwargs: Keyword arguments added to the record
        """
        self.log(logging.INFO, message, **kwargs)

    def debug(self, message: str, **kwargs):
        """Log a message at the debug level

        Args:
            message: Message content, can contain '{name}' where name is a keyword argument given to this function (see kwargs)
            **kwargs: Keyword arguments added to the record
        """
        self.log(logging.DEBUG, message, **kwargs)

    def warning(self, message: str, **kwargs):
        """Log a message at the warning level

        Args:
            message: Message content, can contain '{name}' where name is a keyword argument given to this function (see kwargs)
            **kwargs: Keyword arguments added to the record
        """
        self.log(logging.WARNING, message, **kwargs)

    def error(self, message: str, **kwargs):
        """Log a message at the error level

        Args:
            message: Message content, can contain '{name}' where name is a keyword argument given to this function (see kwargs)
            **kwargs: Keyword arguments added to the record
        """
        self.log(logging.ERROR, message, **kwargs)

    def critical(self, message: str, **kwargs):
        """Log a message at the critical level

        Args:
            message: Message content, can contain '{name}' where name is a keyword argument given to this function (see kwargs)
            **kwargs: Keyword arguments added to the record
        """
        self.log(logging.CRITICAL, message, **kwargs)

    def exception(
        self,
        message: str,
        exc_info: Union[
            BaseException,
            Tuple[Type[BaseException], BaseException, Optional[TracebackType]],
            bool,
            None,
        ] = None,
        **kwargs,
    ):
        """Log a message at the error level with an optional exc_info

        Args:
            message: Message content, can contain '{name}' where name is a keyword argument given to this function (see kwargs)
            exc_info: Optional exc_info to add to an exception record, can be an exception, a boolean (to add the result of sys.exc_info() automatically if true), or the tuple given by sys.exc_info()
            **kwargs: Keyword arguments added to the record
        """
        self.log(logging.ERROR, message, exc_info=exc_info, **kwargs)

    def log(
        self,
        level: int,
        message: str,
        exc_info: Union[
            BaseException,
            Union[
                Tuple[
                    Union[Type[BaseException], None],
                    Union[BaseException, None],
                    Optional[TracebackType],
                ],
                None,
            ],
            bool,
            None,
        ] = None,
        **kwargs,
    ):
        """Log a message

        Args:
            level: level of the message
            message: Message content, can contain '{name}' where name is an keyword argument given to this function (see kwargs)
            exc_info: Optional exc_info to add to an exception record, can be an exception, a boolean (to add the result of sys.exc_info() automatically if true), or the tuple given by sys.exc_info()
            **kwargs: Keyword arguments added to the record
        """
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
                    payload["@x"] = "\n".join(traceback.format_exception(*exc_info))
                for k, v in kwargs.items():
                    if k.startswith("@"):
                        k = "@" + k
                    payload[k] = str(v)
                self._logger.log(level, msg=json.dumps(payload))
        except Exception as e:
            print(f"Couldn't log message : {e}")

    def setLevel(self, level: int):
        """Sets the level of this logger

        Args:
            level: Logging level
        """
        self._logger.setLevel(level)
