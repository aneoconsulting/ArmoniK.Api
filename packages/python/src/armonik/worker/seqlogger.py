from logging import Logger, getLogger, Formatter
from seqlog import ConsoleStructuredLogHandler
from typing import Dict

_worker_loggers: Dict[str, Logger] = {}


def get_worker_logger(name: str, level: int) -> Logger:
    if name in _worker_loggers:
        return _worker_loggers[name]
    logger = getLogger(name)
    logger.handlers.clear()
    handler = ConsoleStructuredLogHandler()
    handler.setFormatter(Formatter(style="{"))
    logger.addHandler(handler)
    logger.setLevel(level)
    _worker_loggers[name] = logger
    return _worker_loggers[name]
