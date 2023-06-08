some_dict = {
    "a": 12,
    "b": 32,
    "c": 44
}

for _, value in some_dict.items():  # W8102
    print(value)


for key, _ in some_dict.items():  # W8102
    print(key)


for weird_arg_name, _ in some_dict.items():  # W8102
    print(weird_arg_name)


for name, (_, _) in some_dict.items():  # W8102
    pass


for name, (value1, _) in some_dict.items():  # OK
    pass


for ((key1, _), (_, _)) in some_dict.items():  # W8102
    pass


for ((_, _), (value, _)) in some_dict.items():  # W8102
    pass


for ((_, key2), (value1, _)) in some_dict.items():  # OK
    pass


for ((_, _, _, variants), (r_language, _, _, _)) in some_dict.items():  # OK
    pass


for key, value in some_dict.items():  # OK
    print(key, value)


for _, value in some_dict.items(12):  # OK
    print(value)


for key in some_dict.keys():  # OK
    print(key)


for value in some_dict.values():  # OK
    print(value)
