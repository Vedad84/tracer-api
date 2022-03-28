from web3 import Web3, HTTPProvider
from abc import ABC
from utils.log_report import LogForReporter

log = LogForReporter(__name__).logger


class RpcConnection(Web3, ABC):

    def __init__(self, end_point: HTTPProvider):
        web3 = Web3(Web3.HTTPProvider(end_point))
        super().__init__()
        self.__class__ = type(web3.__class__.__name__,
                              (self.__class__, web3.__class__),
                              {})

        self.__dict__ = web3.__dict__
        self.__connection_status()

    def __connection_status(self):
        log.info('Web3 connecting...')

        if not (con_status := self.isConnected()):
            log.error(f'Failed to connect over Web3 --- {self.provider}')
            raise Exception('Failed to create connection.')
        log.info(f'Web3 connected {self.provider} {con_status}')
        return
