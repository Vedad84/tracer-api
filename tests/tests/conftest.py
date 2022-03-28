

def pytest_addoption(parser):
    parser.addoption('--end_point', action='store')
    parser.addoption('--target', action='store', default='http://localhost:8545')
