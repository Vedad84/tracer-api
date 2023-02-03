from types import GeneratorType


def find_values_in_dict_for_key(key_to_look: str, data_to_check: dict) -> GeneratorType:
    """
    Generator that iterates over provided dict and finds all values for provided key
    Args:
        key_to_look: key value
        data_to_check: dict to work with

    Returns: value found by key

    """
    def find_value_in_dict(data):
        for dict_key, dict_value in data.items():
            if dict_key == key_to_look:
                yield dict_value

            elif isinstance(dict_value, list):
                yield from find_value_in_list_tuple(dict_value)
            elif isinstance(dict_value, dict):
                if nested_search := find_value_in_dict(dict_value):
                    yield from nested_search
                else:
                    continue

    def find_value_in_list_tuple(data):
        for data_list in data:
            if isinstance(data_list, list):
                if nested_search := find_value_in_list_tuple(data_list):
                    yield from nested_search
                else:
                    continue

            elif isinstance(data_list, dict):
                if nested_search := find_value_in_dict(data_list):
                    yield from nested_search
                else:
                    continue

    if type(data_to_check) is dict:
        yield from find_value_in_dict(data_to_check)

    elif type(data_to_check) is list or type(key_to_look) is tuple:
        yield from find_value_in_list_tuple(data_to_check)

    elif type(data_to_check) is str or type(data_to_check) is int:
        if key_to_look == data_to_check:
            yield key_to_look

    elif data_to_check is None:
        raise Exception("There is no way to check value by key in 'None' ¯\_(ツ)_/¯")