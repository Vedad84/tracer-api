import sys
from betterconf import Config


class TestConfig(dict):
    """
    Parse input args and data from configuration file.
    """
    __args = {option.strip('--'): value for (option, value) in
              [k.split('=') for k in sys.argv if
               ('--target' in k)]}

    endpoint = __args.get('target', 'http://localhost:8545')


cfg = TestConfig()
