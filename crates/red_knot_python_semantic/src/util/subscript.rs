use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct OutOfBoundsError;

pub(crate) trait PyIndex {
    type Item;

    fn py_index(&mut self, index: i64) -> Result<Self::Item, OutOfBoundsError>;
}

enum Nth {
    FromStart(usize),
    FromEnd(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ExceedsUsizeBoundsError;

fn from_nonnegative_i64(index: i64) -> Result<usize, ExceedsUsizeBoundsError> {
    usize::try_from(index).map_err(|_| ExceedsUsizeBoundsError)
}

fn from_negative_i64(index: i64) -> Result<usize, ExceedsUsizeBoundsError> {
    Ok(index
        .checked_neg()
        .map(from_nonnegative_i64)
        .transpose()?
        .unwrap_or({
            // The 'checked_neg' above only fails for i64::MIN. We can not
            // represent -i64::MIN as a i64, but we can represent it as a
            // usize (on 64bit platforms):
            from_nonnegative_i64(i64::MAX)? + 1
        }))
}

impl Nth {
    fn from_index(index: i64) -> Result<Self, ExceedsUsizeBoundsError> {
        if index >= 0 {
            Ok(Nth::FromStart(from_nonnegative_i64(index)?))
        } else {
            Ok(Nth::FromEnd(from_negative_i64(index)? - 1))
        }
    }

    fn to_nonnegative_index(&self, len: usize) -> usize {
        match self {
            Nth::FromStart(nth) => *nth,
            Nth::FromEnd(nth_rev) => len - (*nth_rev).min(len - 1) - 1,
        }
    }
}

impl<I, T> PyIndex for T
where
    T: DoubleEndedIterator<Item = I>,
{
    type Item = I;

    fn py_index(&mut self, index: i64) -> Result<I, OutOfBoundsError> {
        match Nth::from_index(index).map_err(|ExceedsUsizeBoundsError| OutOfBoundsError)? // TODO: this is an assumption that might be unjustified
        {
            Nth::FromStart(nth) => self.nth(nth).ok_or(OutOfBoundsError),
            Nth::FromEnd(nth_rev) => self.rev().nth(nth_rev).ok_or(OutOfBoundsError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct StepSizeZeroError;

pub(crate) trait PySlice {
    type Item;

    fn py_slice<'a>(
        &'a mut self,
        start: Option<i64>,
        stop: Option<i64>,
        step: Option<i64>,
    ) -> Result<Box<dyn Iterator<Item = Self::Item> + 'a>, StepSizeZeroError>
    where
        Self::Item: 'a;
}

impl<I, T> PySlice for T
where
    T: DoubleEndedIterator<Item = I> + ExactSizeIterator<Item = I> + Iterator<Item = I>,
{
    type Item = I;

    fn py_slice<'a>(
        &'a mut self,
        start: Option<i64>,
        stop: Option<i64>,
        step_int: Option<i64>,
    ) -> Result<Box<dyn Iterator<Item = I> + 'a>, StepSizeZeroError>
    where
        I: 'a,
    {
        let len = self.len();

        if len == 0 {
            return Ok(Box::new(std::iter::empty()));
        }

        let step_int = step_int.unwrap_or(1);
        if step_int == 0 {
            return Err(StepSizeZeroError);
        }

        if step_int > 0 {
            let start = start
                .map(Nth::from_index)
                .transpose()
                .unwrap() // TODO
                .map(|start| start.to_nonnegative_index(len))
                .unwrap_or(0)
                .clamp(0, len);
            let stop = stop
                .map(Nth::from_index)
                .transpose()
                .unwrap() // TODO
                .map(|stop| stop.to_nonnegative_index(len))
                .unwrap_or(len)
                .clamp(0, len);

            let step = from_nonnegative_i64(step_int).unwrap(); // TODO
            let (skip, take_n, step) = match start.cmp(&stop) {
                Ordering::Equal => (start, 0, step),
                Ordering::Less => (start, stop - start, step),
                Ordering::Greater => (stop + 1, 0, step),
            };

            Ok(Box::new(self.skip(skip).take(take_n).step_by(step)))
        } else {
            let start = start
                .map(Nth::from_index)
                .transpose()
                .unwrap() // TODO
                .map(|start| start.to_nonnegative_index(len))
                .unwrap_or(len)
                .clamp(0, len - 1);
            let stop = stop
                .map(Nth::from_index)
                .transpose()
                .unwrap() // TODO
                .map(|stop| stop.to_nonnegative_index(len))
                .map(|index| index.clamp(0, len - 1));

            let step = from_negative_i64(step_int).unwrap(); // TODO

            let (skip, take_n, step) = if let Some(stop) = stop {
                match start.cmp(&stop) {
                    Ordering::Equal => (start, 0, step),
                    Ordering::Less => (len - start, 0, step),
                    Ordering::Greater => ((len - 1) - start, start - stop, step),
                }
            } else {
                ((len - 1) - start, len, step)
            };

            Ok(Box::new(self.rev().skip(skip).take(take_n).step_by(step)))
        }
    }
}

#[cfg(test)]
#[allow(clippy::redundant_clone)]
mod tests {
    use crate::util::subscript::{OutOfBoundsError, StepSizeZeroError};

    use super::{PyIndex, PySlice};
    use itertools::assert_equal;

    #[test]
    fn py_index_empty() {
        let iter = std::iter::empty::<char>();

        assert_eq!(iter.clone().py_index(0), Err(OutOfBoundsError));
        assert_eq!(iter.clone().py_index(1), Err(OutOfBoundsError));
        assert_eq!(iter.clone().py_index(-1), Err(OutOfBoundsError));
        assert_eq!(iter.clone().py_index(i64::MIN), Err(OutOfBoundsError));
        assert_eq!(iter.clone().py_index(i64::MAX), Err(OutOfBoundsError));
    }

    #[test]
    fn py_index_single_element() {
        let iter = ['a'].into_iter();

        assert_eq!(iter.clone().py_index(0), Ok('a'));
        assert_eq!(iter.clone().py_index(1), Err(OutOfBoundsError));
        assert_eq!(iter.clone().py_index(-1), Ok('a'));
        assert_eq!(iter.clone().py_index(-2), Err(OutOfBoundsError));
    }

    #[test]
    fn py_index_more_elements() {
        let iter = ['a', 'b', 'c', 'd', 'e'].into_iter();

        assert_eq!(iter.clone().py_index(0), Ok('a'));
        assert_eq!(iter.clone().py_index(1), Ok('b'));
        assert_eq!(iter.clone().py_index(4), Ok('e'));
        assert_eq!(iter.clone().py_index(5), Err(OutOfBoundsError));

        assert_eq!(iter.clone().py_index(-1), Ok('e'));
        assert_eq!(iter.clone().py_index(-2), Ok('d'));
        assert_eq!(iter.clone().py_index(-5), Ok('a'));
        assert_eq!(iter.clone().py_index(-6), Err(OutOfBoundsError));
    }

    #[test]
    fn py_index_uses_full_index_range() {
        let iter = 0..=u64::MAX;

        // u64::MAX - |i64::MIN| + 1 = 2^64 - 1 - 2^63 + 1 = 2^63
        assert_eq!(iter.clone().py_index(i64::MIN), Ok(2u64.pow(63)));
        assert_eq!(iter.clone().py_index(-2), Ok(u64::MAX - 2 + 1));
        assert_eq!(iter.clone().py_index(-1), Ok(u64::MAX - 1 + 1));

        assert_eq!(iter.clone().py_index(0), Ok(0));
        assert_eq!(iter.clone().py_index(1), Ok(1));
        assert_eq!(iter.clone().py_index(i64::MAX), Ok(i64::MAX as u64));
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn py_index_uses_full_index_range_32bit() {
        let iter = 0..=u64::MAX;

        assert_eq!(
            iter.clone().py_index(i64::MAX),
            Err(OutOfBoundsError::ExceedsUsizeBounds)
        );
        assert_eq!(
            iter.clone().py_index(i64::MIN),
            Err(OutOfBoundsError::ExceedsUsizeBounds)
        );
    }

    #[track_caller]
    fn assert_eq_slice<const N: usize, const M: usize>(
        input: &[char; N],
        start: Option<i64>,
        stop: Option<i64>,
        step: Option<i64>,
        expected: &[char; M],
    ) {
        assert_equal(
            input.iter().py_slice(start, stop, step).unwrap(),
            expected.iter(),
        );
    }

    #[track_caller]
    fn assert_slice_returns_none<const N: usize>(
        input: &[char; N],
        start: Option<i64>,
        stop: Option<i64>,
        step: Option<i64>,
    ) {
        assert!(matches!(
            input.iter().py_slice(start, stop, step),
            Err(StepSizeZeroError)
        ));
    }

    #[test]
    fn py_slice_empty_input() {
        let input = [];

        assert_eq_slice(&input, None, None, None, &[]);
        assert_eq_slice(&input, Some(0), None, None, &[]);
        assert_eq_slice(&input, None, Some(0), None, &[]);
        assert_eq_slice(&input, Some(0), Some(0), None, &[]);
        assert_eq_slice(&input, Some(-5), Some(-5), None, &[]);
        assert_eq_slice(&input, None, None, Some(-1), &[]);
        assert_eq_slice(&input, None, None, Some(2), &[]);
    }

    #[test]
    fn py_slice_single_element_input() {
        let input = ['a'];

        assert_eq_slice(&input, None, None, None, &['a']);

        assert_eq_slice(&input, Some(0), None, None, &['a']);
        assert_eq_slice(&input, None, Some(0), None, &[]);
        assert_eq_slice(&input, Some(0), Some(0), None, &[]);
        assert_eq_slice(&input, Some(0), Some(1), None, &['a']);
        assert_eq_slice(&input, Some(0), Some(2), None, &['a']);

        assert_eq_slice(&input, Some(-1), None, None, &['a']);
        assert_eq_slice(&input, Some(-1), Some(-1), None, &[]);
        assert_eq_slice(&input, Some(-1), Some(0), None, &[]);
        assert_eq_slice(&input, Some(-1), Some(1), None, &['a']);
        assert_eq_slice(&input, Some(-1), Some(2), None, &['a']);
        assert_eq_slice(&input, None, Some(-1), None, &[]);

        assert_eq_slice(&input, Some(-2), None, None, &['a']);
        assert_eq_slice(&input, Some(-2), Some(-1), None, &[]);
        assert_eq_slice(&input, Some(-2), Some(0), None, &[]);
        assert_eq_slice(&input, Some(-2), Some(1), None, &['a']);
        assert_eq_slice(&input, Some(-2), Some(2), None, &['a']);
    }

    #[test]
    fn py_slice_nonnegative_indices() {
        let input = ['a', 'b', 'c', 'd', 'e'];

        assert_eq_slice(&input, None, Some(0), None, &[]);
        assert_eq_slice(&input, None, Some(1), None, &['a']);
        assert_eq_slice(&input, None, Some(4), None, &['a', 'b', 'c', 'd']);
        assert_eq_slice(&input, None, Some(5), None, &['a', 'b', 'c', 'd', 'e']);
        assert_eq_slice(&input, None, Some(6), None, &['a', 'b', 'c', 'd', 'e']);
        assert_eq_slice(&input, None, None, None, &['a', 'b', 'c', 'd', 'e']);

        assert_eq_slice(&input, Some(0), Some(0), None, &[]);
        assert_eq_slice(&input, Some(0), Some(1), None, &['a']);
        assert_eq_slice(&input, Some(0), Some(4), None, &['a', 'b', 'c', 'd']);
        assert_eq_slice(&input, Some(0), Some(5), None, &['a', 'b', 'c', 'd', 'e']);
        assert_eq_slice(&input, Some(0), Some(6), None, &['a', 'b', 'c', 'd', 'e']);
        assert_eq_slice(&input, Some(0), None, None, &['a', 'b', 'c', 'd', 'e']);

        assert_eq_slice(&input, Some(1), Some(0), None, &[]);
        assert_eq_slice(&input, Some(1), Some(1), None, &[]);
        assert_eq_slice(&input, Some(1), Some(2), None, &['b']);
        assert_eq_slice(&input, Some(1), Some(4), None, &['b', 'c', 'd']);
        assert_eq_slice(&input, Some(1), Some(5), None, &['b', 'c', 'd', 'e']);
        assert_eq_slice(&input, Some(1), Some(6), None, &['b', 'c', 'd', 'e']);
        assert_eq_slice(&input, Some(1), None, None, &['b', 'c', 'd', 'e']);

        assert_eq_slice(&input, Some(4), Some(0), None, &[]);
        assert_eq_slice(&input, Some(4), Some(4), None, &[]);
        assert_eq_slice(&input, Some(4), Some(5), None, &['e']);
        assert_eq_slice(&input, Some(4), Some(6), None, &['e']);
        assert_eq_slice(&input, Some(4), None, None, &['e']);

        assert_eq_slice(&input, Some(5), Some(0), None, &[]);
        assert_eq_slice(&input, Some(5), Some(5), None, &[]);
        assert_eq_slice(&input, Some(5), Some(6), None, &[]);
        assert_eq_slice(&input, Some(5), None, None, &[]);

        assert_eq_slice(&input, Some(6), Some(0), None, &[]);
        assert_eq_slice(&input, Some(6), Some(6), None, &[]);
        assert_eq_slice(&input, Some(6), None, None, &[]);
    }

    #[test]
    fn py_slice_negatice_indices() {
        let input = ['a', 'b', 'c', 'd', 'e'];

        assert_eq_slice(&input, Some(-6), None, None, &['a', 'b', 'c', 'd', 'e']);
        assert_eq_slice(&input, Some(-6), Some(-1), None, &['a', 'b', 'c', 'd']);
        assert_eq_slice(&input, Some(-6), Some(-4), None, &['a']);
        assert_eq_slice(&input, Some(-6), Some(-5), None, &[]);
        assert_eq_slice(&input, Some(-6), Some(-6), None, &[]);
        assert_eq_slice(&input, Some(-6), Some(-10), None, &[]);

        assert_eq_slice(&input, Some(-5), None, None, &['a', 'b', 'c', 'd', 'e']);
        assert_eq_slice(&input, Some(-5), Some(-1), None, &['a', 'b', 'c', 'd']);
        assert_eq_slice(&input, Some(-5), Some(-4), None, &['a']);
        assert_eq_slice(&input, Some(-5), Some(-5), None, &[]);
        assert_eq_slice(&input, Some(-5), Some(-6), None, &[]);
        assert_eq_slice(&input, Some(-5), Some(-10), None, &[]);

        assert_eq_slice(&input, Some(-4), None, None, &['b', 'c', 'd', 'e']);
        assert_eq_slice(&input, Some(-4), Some(-1), None, &['b', 'c', 'd']);
        assert_eq_slice(&input, Some(-4), Some(-3), None, &['b']);
        assert_eq_slice(&input, Some(-4), Some(-4), None, &[]);
        assert_eq_slice(&input, Some(-4), Some(-10), None, &[]);

        assert_eq_slice(&input, Some(-1), None, None, &['e']);
        assert_eq_slice(&input, Some(-1), Some(-1), None, &[]);
        assert_eq_slice(&input, Some(-1), Some(-10), None, &[]);

        assert_eq_slice(&input, None, Some(-1), None, &['a', 'b', 'c', 'd']);
        assert_eq_slice(&input, None, Some(-4), None, &['a']);
        assert_eq_slice(&input, None, Some(-5), None, &[]);
        assert_eq_slice(&input, None, Some(-6), None, &[]);
    }

    #[test]
    fn py_slice_mixed_positive_negative_indices() {
        let input = ['a', 'b', 'c', 'd', 'e'];

        assert_eq_slice(&input, Some(0), Some(-1), None, &['a', 'b', 'c', 'd']);
        assert_eq_slice(&input, Some(1), Some(-1), None, &['b', 'c', 'd']);
        assert_eq_slice(&input, Some(3), Some(-1), None, &['d']);
        assert_eq_slice(&input, Some(4), Some(-1), None, &[]);
        assert_eq_slice(&input, Some(5), Some(-1), None, &[]);

        assert_eq_slice(&input, Some(0), Some(-4), None, &['a']);
        assert_eq_slice(&input, Some(1), Some(-4), None, &[]);
        assert_eq_slice(&input, Some(3), Some(-4), None, &[]);

        assert_eq_slice(&input, Some(0), Some(-5), None, &[]);
        assert_eq_slice(&input, Some(1), Some(-5), None, &[]);
        assert_eq_slice(&input, Some(3), Some(-5), None, &[]);

        assert_eq_slice(&input, Some(0), Some(-6), None, &[]);
        assert_eq_slice(&input, Some(1), Some(-6), None, &[]);

        assert_eq_slice(&input, Some(-6), Some(6), None, &['a', 'b', 'c', 'd', 'e']);
        assert_eq_slice(&input, Some(-6), Some(5), None, &['a', 'b', 'c', 'd', 'e']);
        assert_eq_slice(&input, Some(-6), Some(4), None, &['a', 'b', 'c', 'd']);
        assert_eq_slice(&input, Some(-6), Some(1), None, &['a']);
        assert_eq_slice(&input, Some(-6), Some(0), None, &[]);

        assert_eq_slice(&input, Some(-5), Some(6), None, &['a', 'b', 'c', 'd', 'e']);
        assert_eq_slice(&input, Some(-5), Some(5), None, &['a', 'b', 'c', 'd', 'e']);
        assert_eq_slice(&input, Some(-5), Some(4), None, &['a', 'b', 'c', 'd']);
        assert_eq_slice(&input, Some(-5), Some(1), None, &['a']);
        assert_eq_slice(&input, Some(-5), Some(0), None, &[]);

        assert_eq_slice(&input, Some(-4), Some(6), None, &['b', 'c', 'd', 'e']);
        assert_eq_slice(&input, Some(-4), Some(5), None, &['b', 'c', 'd', 'e']);
        assert_eq_slice(&input, Some(-4), Some(4), None, &['b', 'c', 'd']);
        assert_eq_slice(&input, Some(-4), Some(2), None, &['b']);
        assert_eq_slice(&input, Some(-4), Some(1), None, &[]);
        assert_eq_slice(&input, Some(-4), Some(0), None, &[]);

        assert_eq_slice(&input, Some(-1), Some(6), None, &['e']);
        assert_eq_slice(&input, Some(-1), Some(5), None, &['e']);
        assert_eq_slice(&input, Some(-1), Some(4), None, &[]);
        assert_eq_slice(&input, Some(-1), Some(1), None, &[]);
    }

    #[test]
    fn py_slice_step_forward() {
        // indices:   0    1    2    3    4    5    6
        let input = ['a', 'b', 'c', 'd', 'e', 'f', 'g'];

        // Step size zero is invalid:
        assert_slice_returns_none(&input, Some(0), Some(5), Some(0));
        assert_slice_returns_none(&input, Some(0), Some(0), Some(0));

        assert_eq_slice(&input, Some(0), Some(8), Some(2), &['a', 'c', 'e', 'g']);
        assert_eq_slice(&input, Some(0), Some(7), Some(2), &['a', 'c', 'e', 'g']);
        assert_eq_slice(&input, Some(0), Some(6), Some(2), &['a', 'c', 'e']);
        assert_eq_slice(&input, Some(0), Some(5), Some(2), &['a', 'c', 'e']);
        assert_eq_slice(&input, Some(0), Some(4), Some(2), &['a', 'c']);
        assert_eq_slice(&input, Some(0), Some(3), Some(2), &['a', 'c']);
        assert_eq_slice(&input, Some(0), Some(2), Some(2), &['a']);
        assert_eq_slice(&input, Some(0), Some(1), Some(2), &['a']);
        assert_eq_slice(&input, Some(0), Some(0), Some(2), &[]);
        assert_eq_slice(&input, Some(1), Some(5), Some(2), &['b', 'd']);

        assert_eq_slice(&input, Some(0), Some(7), Some(3), &['a', 'd', 'g']);
        assert_eq_slice(&input, Some(0), Some(6), Some(3), &['a', 'd']);
    }

    #[test]
    fn py_slice_step_backward() {
        // indices:   0    1    2    3    4    5    6
        let input = ['a', 'b', 'c', 'd', 'e', 'f', 'g'];

        assert_eq_slice(&input, Some(7), Some(0), Some(-2), &['g', 'e', 'c']);
        assert_eq_slice(&input, Some(6), Some(0), Some(-2), &['g', 'e', 'c']);
        assert_eq_slice(&input, Some(5), Some(0), Some(-2), &['f', 'd', 'b']);
        assert_eq_slice(&input, Some(4), Some(0), Some(-2), &['e', 'c']);
        assert_eq_slice(&input, Some(3), Some(0), Some(-2), &['d', 'b']);
        assert_eq_slice(&input, Some(2), Some(0), Some(-2), &['c']);
        assert_eq_slice(&input, Some(1), Some(0), Some(-2), &['b']);
        assert_eq_slice(&input, Some(0), Some(0), Some(-2), &[]);

        assert_eq_slice(&input, Some(7), None, Some(-2), &['g', 'e', 'c', 'a']);
        assert_eq_slice(&input, None, None, Some(-2), &['g', 'e', 'c', 'a']);
        assert_eq_slice(&input, None, Some(0), Some(-2), &['g', 'e', 'c']);

        assert_eq_slice(&input, Some(5), Some(1), Some(-2), &['f', 'd']);
        assert_eq_slice(&input, Some(5), Some(2), Some(-2), &['f', 'd']);
        assert_eq_slice(&input, Some(5), Some(3), Some(-2), &['f']);
        assert_eq_slice(&input, Some(5), Some(4), Some(-2), &['f']);
        assert_eq_slice(&input, Some(5), Some(5), Some(-2), &[]);
    }
}
