import json
import os
from parse_args import cfg

ROOT_DIR = os.path.dirname(os.path.abspath(__file__))
RESOURCES = f'{ROOT_DIR}/../resources/'

url = cfg.endpoint
url_trace = cfg.trace_url


def read_saved_tx_hashes_level() -> dict:
    with open(f'{RESOURCES}/transactions_level.json') as file:
        tx_data = file.read()
        return json.loads(tx_data)


class TestParameterizationHelper:
    """
    Class contain methods that helps with tests parameterization.
    """

    def __init__(self):
        ...

    @staticmethod
    def test_name_with_parameter(testcase_func, _, param) -> str:
        """
        Format test method(test name) to contain parameters values.

        Args:
            testcase_func: test function.
            param: test function parameters.

        Returns:
            Formatted test name string.
        """
        return f"{str(testcase_func.__name__)}_{'_'.join(param.args[3])}"


level_test_parameters = {'input': [[tx_hash, url, url_trace, role] for role, tx_hash in read_saved_tx_hashes_level().items()],
                         'name_func': TestParameterizationHelper.test_name_with_parameter}

