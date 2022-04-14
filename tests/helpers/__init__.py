from termcolor import colored
from functools import partial


ALAN_PER_NEON = 10 ** 18
GALAN_PER_NEON = 10 ** 9

failed_text = partial(colored, color='red', attrs=['bold'])
failed_text_under_line = partial(colored, color='red', attrs=['bold', 'underline'])
blue_under_line = partial(colored, color='blue', attrs=['underline'])
yellow_text = partial(colored, color='yellow', attrs=['bold'])
green_text = partial(colored, color='green', attrs=['bold'])
blue_text = partial(colored, color='blue', attrs=['bold'])
