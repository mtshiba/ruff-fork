# These SHOULD change
'%s %s' % (a, b)

"trivial" % ()

"%s" % ("simple",)

"%s" % ("%s" % ("nested",),)

"%s%% percent" % (15,)

"%3f" % (15,)

"%-5f" % (5,)

"%9f" % (5,)

"brace {} %s" % (1,)

"%s" % (
    "trailing comma",
        )

""" Having this uncommented breaks the nested one, waiting on help from Charlie to uncomment this
"%s \N{snowman}" % (a,)
"""
# Make sure to include assignment and inside a call, also multi-line

# These SHOULD NOT change
"%s" % unknown_type

b"%s" % (b"bytestring",)

"%*s" % (5, "hi")

"%d" % (flt,)

"%c" % (some_string,)

"%#o" % (123,)

"%()s" % {"": "empty"}

"%4%" % ()

"%.2r" % (1.25)

i % 3

"%s" % {"k": "v"}

"%(1)s" % {"1": "bar"}

"%(a)s" % {"a": 1, "a": 2}

"%(ab)s" % {"a" "b": 1}

"%(a)s" % {"a"  :  1}

"%(1)s" % {1: 2, "1": 2}

"%(and)s" % {"and": 2}

"%" % {}

"%()s" % {"": "bar"}

"%.*s" % (5, "hi")

"%i" % (flt,)

pytest.param('"%8s" % (None,)', id='unsafe width-string conversion'),
