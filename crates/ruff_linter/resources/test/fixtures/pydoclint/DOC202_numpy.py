# OK
def foo(num: int) -> str:
    """
    Do something

    Parameters
    ----------
    num : int
        A number
    """
    print('test')


# DOC202
def foo(num: int) -> str:
    """
    Do something

    Parameters
    ----------
    num : int
        A number

    Returns
    -------
    str
        A string
    """
    print('test')


class Bar:

    # DOC202
    def foo(self) -> str:
        """
        Do something

        Parameters
        ----------
        num : int
            A number

        Returns
        -------
        str
            A string
        """
        print('test')


    # OK
    def bar(self) -> str:
        """
        Do something

        Parameters
        ----------
        num : int
            A number
        """
        print('test')
