input = [1, 2, 3]
otherInput = [2, 3, 4]

# Don't trigger
zip(input, otherInput)  # different inputs
zip(input, otherInput[1:])  # different inputs
zip(input, input[2:])  # not successive
zip(input[:-1], input[2:])  # not successive
list(zip(input, otherInput))  # nested call

# Error - prefer pairwise here
zip(input, input[1:])
zip(input[:-1], input[1:])
zip(input[1:], input[2:])
zip(input[1:-1], input[2:])
list(zip(input, input[1:]))
list(zip(input[:-1], input[1:]))
