import sys
import six

if six.PY2:
    print("py2")
else:
    print("py3")

if six.PY2:
    if True:
        print("py2!")
    else:
        print("???")
else:
    print("py3")

if six.PY2: print("PY2!")
else: print("PY3!")

if True:
    if six.PY2:
        print("PY2")
    else:
        print("PY3")

if six.PY2: print(1 if True else 3)
else:
    print("py3")

if six.PY2:
    def f():
        print("py2")
else:
    def f():
        print("py3")
        print("This the next")

if not six.PY2:
    print("py3")
else:
    print("py2")
