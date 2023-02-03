"""
Implements one form of delayed assertions.
Interface is 2 functions:
  expect(expr, msg=None)
  : Evaluate 'expr' as a boolean, and keeps track of failures
  assert_expectations()
  : raises an assert if an expect() calls failed
Usage Example:
    from delayed_assert import expect, assert_expectations
    def test_should_pass():
        expect(1 == 1, 'one is one')
        assert_expectations()
    def test_should_fail():
        expect(1 == 2, 'one is two')
        expect(1 == 3, 'one is three')
        assert_expectations()

    https://github.com/rackerlabs/python-proboscis/blob/master/proboscis/check.py
"""# pylint: skip-file
# TODO one day I will refactor this.

import types
import inspect
from contextlib import contextmanager
import locale
import sys
# from .print_decorator.print_utils import table
from helpers.helper import find_values_in_dict_for_key
from jsonschema import ValidationError, SchemaError, Draft3Validator
from helpers import  failed_text, failed_text_under_line, blue_under_line

_failed_expectations = []

# failed_text = partial(colored, color='red', attrs=['bold'])
# failed_text_under_line = partial(colored, color='red', attrs=['bold', 'underline'])
# blue_under_line = partial(colored, color='blue', attrs=['underline'])


def soft_assert(expr, msg=None, data_info=None):
    """Keeps track of failed expectations"""
    # global _failed_expectations
    if isinstance(expr, types.FunctionType):
        try:
            expr()
        except Exception as e:
            _log_failure(e)
    elif not expr:
        _log_failure(msg, data_info)
        print(f'\n\t{failed_text(msg)}\n')


def soft_assert_type(data, schema_to_compare: dict) -> bool:
    try:
        Draft3Validator(schema_to_compare).validate(data)
        if type(data) is list and not len(data):
            raise ValidationError(f'Schema can not be validated against empty data "{data}"')
    except ValidationError as ae:
        if len(ae.path):
            failed_key = ae.path.pop()
            failed_value = ae.instance
            if type(data) is list:
                what_failed = next(item for item in data if item[failed_key] == failed_value)
                del data[:]
                data.append(what_failed)
            elif type(data) is dict:
                print("ae", ae)
                print(filed_key)
                print(data)
                what_failed = next(find_values_in_dict_for_key(failed_key, data))
                data.clear()
                data.update({failed_key: what_failed})
        _log_failure(ae)
        print(ae)
        return False
    except SchemaError as e:
        _log_failure(e)
        return False
    return True

# def soft_assert_generic_type(actual, matcher, msg=None, data_info=None):
#     try:
#         assert_that(actual,  instance_of(matcher), msg)
#     except AssertionError as ae:
#         _log_failure(ae, data_info)


def assert_expectations():
    """Raise an assert if there are any failed expectations"""
    locale.getpreferredencoding()
    if _failed_expectations:
        sys.stderr.write(_report_failures())
        raise AssertionError("Check log for 'Captured stderr' for list of assertion error")
        # assert False, "Check log for 'Captured stderr'"


@contextmanager
def assert_all():
    # try:
    yield
    # except Exception:
    #     pass  # if we face some exception before assertion for some reason.
    assert_expectations()


def _log_failure(msg=None, data_info=None):
    (file_path, line, func_name, context_list) = inspect.stack()[2][1:5]

    _failed_expectations.append(
        failed_text('Failed at ') +
        blue_under_line('"{}:{}"'.format(file_path, line)) +
        failed_text(', in {}()\n\t'.format(func_name)) +
        failed_text_under_line('ErrorMessage') +
        failed_text(': {}\n {}'.format(msg, data_info))
    )


def _report_failures():
    global _failed_expectations
    report = []

    if _failed_expectations:
        report += [failed_text('Failed Expectations: {}\n'.format(len(_failed_expectations)))]

        report += ['{}:\t{}'.format(failure_num, failure)
                   for failure_num, failure in enumerate(_failed_expectations, start=1)]

        _failed_expectations = []

    return '\n'.join(report)
