# Too many positional arguments (6/4) for max_positional=4
# OK for dummy_variable_rgx ~ "skip_.*"
def f(x, y, z, skip_t, skip_u, skip_v):
    pass


# Too many positional arguments (6/4) for max_args=4
# Too many positional arguments (6/3) for dummy_variable_rgx ~ "skip_.*"
def f(x, y, z, t, u, v):
    pass
