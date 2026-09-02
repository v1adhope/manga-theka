use std::{fmt, marker::PhantomData};

use serde::{Deserialize, Serialize};

use crate::error::EntityError;

pub trait Bounded {
    const MAX: usize;
    const NAME: &'static str;
}

#[derive(Serialize, Deserialize)]
#[serde(
    transparent,
    bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>")
)]
pub struct BoundedVec<T, B>(Vec<T>, #[serde(skip)] PhantomData<B>);

impl<T: fmt::Debug, B> fmt::Debug for BoundedVec<T, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T, B: Bounded> TryFrom<Vec<T>> for BoundedVec<T, B> {
    type Error = EntityError;

    fn try_from(items: Vec<T>) -> Result<Self, Self::Error> {
        if items.len() > B::MAX {
            return Err(EntityError::CollectionExceedsLimit(
                B::NAME,
                items.len(),
                B::MAX,
            ));
        }

        Ok(Self(items, PhantomData))
    }
}

impl<T: Ord, B: Bounded> BoundedVec<T, B> {
    /// Builds a set: a repeated query parameter ORs within itself, so a duplicate says nothing
    /// the first occurrence did not. Deduplicating before the bound also keeps a caller from
    /// spending the ceiling on repeats.
    pub fn deduped(mut items: Vec<T>) -> Result<Self, EntityError> {
        items.sort_unstable();
        items.dedup();

        Self::try_from(items)
    }
}

impl<T, B> BoundedVec<T, B> {
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{Bounded, BoundedVec};

    struct TestBound;

    impl Bounded for TestBound {
        const MAX: usize = 2;
        const NAME: &'static str = "test items";
    }

    type TestVec = BoundedVec<u8, TestBound>;

    #[test]
    fn at_the_ceiling_is_valid() {
        let res = TestVec::try_from(vec![1, 2]);
        assert!(res.is_ok());
    }

    #[test]
    fn over_the_ceiling_is_rejected() {
        let res = TestVec::try_from(vec![1, 2, 3]);
        assert!(res.is_err());
    }

    #[test]
    fn over_the_ceiling_error_names_the_collection() {
        let res = TestVec::try_from(vec![1, 2, 3]);
        let err = res.unwrap_err().to_string();
        assert!(err.contains("test items"));
        assert!(err.contains('3'));
        assert!(err.contains('2'));
    }

    #[test]
    fn duplicates_collapse_before_the_ceiling_is_measured() {
        let res = TestVec::deduped(vec![1, 1, 2, 2, 1]);

        assert_eq!(res.unwrap().as_slice(), [1, 2]);
    }

    #[test]
    fn deduping_orders_the_set() {
        let res = TestVec::deduped(vec![2, 1]);

        assert_eq!(
            res.unwrap().as_slice(),
            [1, 2],
            "wire order must not reach the filter hash"
        );
    }

    #[test]
    fn distinct_values_over_the_ceiling_are_still_rejected() {
        let res = TestVec::deduped(vec![1, 2, 3]);
        assert!(res.is_err());
    }

    #[test]
    fn len_and_is_empty_reflect_contents() {
        let empty = TestVec::try_from(vec![]).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);

        let one = TestVec::try_from(vec![1]).unwrap();
        assert!(!one.is_empty());
        assert_eq!(one.len(), 1);
    }
}
