from dataclasses import dataclass
from solcx import compile_standard, install_solc, get_installed_solc_versions
import json
from typing import Union, Optional

from utils.log_report import LogForReporter

log = LogForReporter(__name__).logger


@dataclass
class CompiledContract:

    abi: list

    bytecode: str

    opcodes: str

    @classmethod
    def from_dict(cls, data: dict) -> "CompiledContract":
        return cls(
            abi=data["abi"],
            bytecode=data["evm"]["bytecode"]["object"],
            opcodes=data["evm"]["bytecode"]["opcodes"]
        ) if data else None


class CompileSolidity:

    def __init__(self, path_to_file: Union[str, list], sol_ver: str = "0.6.0", save_to_file=False):
        self.solidity_code: dict = {}
        self.compiled_sol: dict[str, CompileSolidity] = {}
        self.sol_ver = sol_ver
        self._compiled_sol_as_dict: dict
        self._install_compiler()
        self._read_file_with_code(path_to_file)
        self._compile(save_to_file)
        self.__parse_compiled()

    def __call__(self, dapp_name: str) -> Optional[CompiledContract]:
        return self.compiled_sol.get(dapp_name)

    def _read_file_with_code(self, path_to_file: Union[str, list]):
        if isinstance(path_to_file, str):
            log.info(f'Reading file with solidity code --- {path_to_file} ')
            with open(path_to_file, "r") as file_with_code:
                self.solidity_code.update({file_with_code.name: {"content": file_with_code.read()}})
        elif isinstance(path_to_file, list):
            for path in path_to_file:
                log.info(f'Reading file with solidity code --- {path} ')
                with open(path, "r") as file_with_code:
                    self.solidity_code.update({file_with_code.name: {"content": file_with_code.read()}})

    def _install_compiler(self):
        installed_sols = get_installed_solc_versions()
        if self.sol_ver not in installed_sols:
            try:
                log.info(f'Installing solidity compiler with solcx --- version {self.sol_ver}')
                install_solc(self.sol_ver)
            except Exception as e:
                log.error('Something happened with solidity compiler installation')
                raise e

    def _write_file_with_compilation(self):
        try:
            with open('compiled_code.json', "w") as file:
                json.dump(self._compiled_sol_as_dict, file)
        except Exception as e:
            log.error('Failed to save compiled sol into file')
            raise e

    def __parse_compiled(self):
        nested_data = [contract_data for contract_file, contract_data
                       in self._compiled_sol_as_dict['contracts'].items()]
        for el in nested_data:
            for name, data in el.items():
                self.compiled_sol.update({name: CompiledContract.from_dict(data)})

    def _compile(self, save: bool = False):
        if not self.solidity_code:
            log.error("Can't compile solidity code if there is no code provided")
            raise Exception("Can't compile solidity code if there is no code provided.")
        try:

            log.info(f'Compiling Solidity code')

            self._compiled_sol_as_dict = compile_standard(
                {
                    "language": "Solidity",
                    "sources": self.solidity_code,
                    "settings": {
                        "outputSelection": {
                            "*": {
                                "*": ["abi", "metadata", "evm.bytecode", "evm.bytecode.sourceMap"]
                            }
                        }
                    },
                },
                solc_version=self.sol_ver,
            )
        except Exception as e:
            log.error('Failed to compile')
            raise e

        log.info('Success')

        self._write_file_with_compilation() if save else ...
