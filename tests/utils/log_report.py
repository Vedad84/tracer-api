import logging
import logging.config


class LogForReporter:
    """
    Simple logger with format configuration.
    """
    def __init__(self, name):
        """
        Logger configuration.

        :param name: python module's name which initialized logger.
        """
        self.logger = logging.getLogger(name)
        self.logger.setLevel(logging.INFO)

        c_handler = logging.StreamHandler()
        c_format = logging.Formatter('%(asctime)s [%(name)s] %(levelname)s: %(message)s', "%Y-%m-%d %H:%M:%S")
        c_handler.setFormatter(c_format)

        self.logger.addHandler(c_handler)