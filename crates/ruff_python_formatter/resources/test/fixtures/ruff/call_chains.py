# Test cases for call chains and optional parentheses, with and without fluent style

raise OsError("") from a.aaaaa(
    aksjdhflsakhdflkjsadlfajkslhfdkjsaldajlahflashdfljahlfksajlhfajfjfsaahflakjslhdfkjalhdskjfa
).a(aaaa)

raise OsError(
    "sökdjffffsldkfjlhsakfjhalsökafhsöfdahsödfjösaaksjdllllllllllllll"
) from a.aaaaa(
    aksjdhflsakhdflkjsadlfajkslhfdkjsaldajlahflashdfljahlfksajlhfajfjfsaahflakjslhdfkjalhdskjfa
).a(
    aaaa
)

a1 = Blog.objects.filter(entry__headline__contains="Lennon").filter(
    entry__pub_date__year=2008
)

a2 = Blog.objects.filter(
    entry__headline__contains="Lennon",
).filter(
    entry__pub_date__year=2008,
)

raise OsError("") from (
    Blog.objects.filter(
        entry__headline__contains="Lennon",
    )
    .filter(
        entry__pub_date__year=2008,
    )
    .filter(
        entry__pub_date__year=2008,
    )
)

raise OsError("sökdjffffsldkfjlhsakfjhalsökafhsöfdahsödfjösaaksjdllllllllllllll") from (
    Blog.objects.filter(
        entry__headline__contains="Lennon",
    )
    .filter(
        entry__pub_date__year=2008,
    )
    .filter(
        entry__pub_date__year=2008,
    )
)

# Break only after calls and indexing
b1 = (
    session.query(models.Customer.id)
    .filter(
        models.Customer.account_id == account_id, models.Customer.email == email_address
    )
    .count()
)

b2 = (
    Blog.objects.filter(
        entry__headline__contains="Lennon",
    )
    .limit_results[:10]
    .filter(
        entry__pub_date__month=10,
    )
)

# Nested call chains
c1 = (
    Blog.objects.filter(
        entry__headline__contains="Lennon",
    ).filter(
        entry__pub_date__year=2008,
    )
    + Blog.objects.filter(
        entry__headline__contains="McCartney",
    )
    .limit_results[:10]
    .filter(
        entry__pub_date__year=2010,
    )
).all()

# Test different cases with trailing end of line comments:
# * fluent style, fits: no parentheses -> ignore the expand_parent
# * fluent style, doesn't fit: break all soft line breaks
# * default, fits: no parentheses
# * default, doesn't fit: parentheses but no soft line breaks

# Fits, either style
d11 = x.e().e().e() #
d12 = (x.e().e().e()) #
d13 = (
    x.e() #
    .e()
    .e()
)

# Doesn't fit, default
d2 = (
    x.e().esadjkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkfsdddd()  #
)

# Doesn't fit, fluent style
d3 = (
    x.e()  #
    .esadjkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk()
    .esadjkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk()
)

# Don't drop the bin op parentheses
e1 = (1 + 2).w().t()
e2 = (1 + 2)().w().t()
e3 = (1 + 2)[1].w().t()

# Treat preserved parentheses correctly
# TODO: The implementation assumes `Parentheses::Preserve`, what's with other
# parentheses?
f1 = a.w().t(1,)
f2 = (a).w().t(1,)

# Indent in the parentheses without breaking
g1 = (
    queryset.distinct().order_by(field.name).values_list(field_name_flat_long_long=True)
)

# TODO(konstin): We currently can't do call chains that is not the outermost expression
if (
    not a()
    .b()
    .cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc()
):
    pass
