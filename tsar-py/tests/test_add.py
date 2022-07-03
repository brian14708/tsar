from tsar import tsar


def test_sum_as_string() -> None:
    assert tsar.sum_as_string(1, 2) == "3"
