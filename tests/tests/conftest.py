
def pytest_addoption(parser):
    parser.addoption('--target', action='store', default='http://localhost:8545')
    parser.addoption('--trace_url', action='store', default='http://localhost:8545')
