import json
import os
from parse_args import cfg

ROOT_DIR = os.path.dirname(os.path.abspath(__file__))
RESOURCES = f'{ROOT_DIR}/../resources/'

url = cfg.endpoint #'http://localhost:8545'


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
        Format test method(test name) to contain patameters values.

        Args:
            testcase_func: test function.
            param: test function parameters.

        Returns:
            Formatted test name string.
        """
        return "%s_%s" % (str(testcase_func.__name__), '_'.join(param.args[2]))


# level_test_parameters = [[tx_hash, url, role] for role, tx_hash in read_saved_tx_hashes_level().items()]
#
# print(level_test_parameters)

level_test_parameters = {'input': [[tx_hash, url, role] for role, tx_hash in read_saved_tx_hashes_level().items()],
                         'name_func': TestParameterizationHelper.test_name_with_parameter}

