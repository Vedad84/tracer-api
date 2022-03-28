"""
RPC API Responses json schema validator.
This helper contains very special validator to check JSON data types in SolanaAPI replies.
Match JSON reply with schema(stored in file).
"""  # pylint: disable=line-too-long
from pathlib import Path
from typing import List, DefaultDict, AnyStr, Optional
from helpers.helper import find_values_in_dict_for_key
import simplejson as json
from helpers.soft_assertion import soft_assert_type
from helpers import failed_text, yellow_text, green_text
import os

# from component_test_support.utils.helpers import find_values_in_dict_for_key
# from component_test_support.utils.pretty_printers import pretty_summary_print_data, \
#     pretty_success_print_data, pretty_error_print_data, pretty_warning_print_text
# from component_test_support.utils.soft_assertion import soft_assert_type

ROOT_DIR = os.path.dirname(os.path.abspath(__file__))
PATH_TO_SCHEMAS = f'{ROOT_DIR}/../resources/schemas'


def validate_type_by_scheme(data_to_validate: List[DefaultDict],
                            method_name: AnyStr,
                            key: Optional[AnyStr] = None) -> None:
    """
    Args:
        data_to_validate: List of dict, dict can be full response/request or part of it like value of key from response.
        method_name: name of the API method, to take file with schema by this name.
        key: Key required, to get nested scheme from 'main' scheme.

    """

    def collect_data_from_schema(schema_name: AnyStr) -> Optional[dict]:
        file_path = '/'.join([PATH_TO_SCHEMAS, f'{schema_name}'])
        try:
            with open(file_path, 'r') as schema:
                return json.load(schema)
        except FileNotFoundError:
            return None

    if schema_to_compare := collect_data_from_schema(method_name):

        schema_to_compare = next(find_values_in_dict_for_key(key, schema_to_compare)) \
            if key and schema_to_compare else schema_to_compare
        print(f'Check schema for {method_name}')

        if soft_assert_type(data_to_validate, schema_to_compare):
            print(green_text('Data types are correct'))
        else:
            print(failed_text(f'Data types are incorrect according to schema\n{data_to_validate}'))
    else:
        print(yellow_text('Warning: There is no schema to compare with. Json data type validation was not done'))
