from typing import Iterable, List, TypeVar

T = TypeVar('T')

def batched(iterable: Iterable[T], n: int) -> Iterable[List[T]]:
    """
    Batches elements from an iterable into lists of size at most 'n'.

    Args:
        iterable : The input iterable.
        n : The batch size.

    Yields:
        A generator yielding batches of elements from the input iterable.
    """
    it = iter(iterable)
    while True:
        batch = []
        try:
            for i in range(n):
                batch.append(next(it))
        except StopIteration:
            if len(batch) > 0:
                yield batch
            break
        yield batch
