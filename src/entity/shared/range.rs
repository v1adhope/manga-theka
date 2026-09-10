use std::{fmt, marker::PhantomData};

use serde::Serialize;

use crate::error::EntityError;

pub trait RangeBound {
    const UPPER_INCLUSIVE: bool;
    const NAME: &'static str;
}

#[derive(Serialize)]
#[serde(bound(serialize = "T: Serialize"))]
pub struct Range<T, B>(Option<T>, Option<T>, #[serde(skip)] PhantomData<B>);

impl<T: fmt::Debug, B> fmt::Debug for Range<T, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Range")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl<T: PartialOrd, B: RangeBound> Range<T, B> {
    pub fn try_new(from: Option<T>, to: Option<T>) -> Result<Self, EntityError> {
        if let (Some(from), Some(to)) = (&from, &to)
            && from > to
        {
            return Err(EntityError::RangeIsInverted(B::NAME));
        }

        Ok(Self(from, to, PhantomData))
    }
}

impl<T, B> Range<T, B> {
    pub fn from(&self) -> Option<&T> {
        self.0.as_ref()
    }

    pub fn to(&self) -> Option<&T> {
        self.1.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::{Range, RangeBound};

    struct TestBound;

    impl RangeBound for TestBound {
        const UPPER_INCLUSIVE: bool = true;
        const NAME: &'static str = "test range";
    }

    type TestRange = Range<i16, TestBound>;

    #[test]
    fn an_open_range_is_valid() {
        let range = TestRange::try_new(None, None).unwrap();

        assert_eq!(range.from(), None);
        assert_eq!(range.to(), None);
    }

    #[test]
    fn either_bound_is_usable_alone() {
        let lower = TestRange::try_new(Some(2020), None).unwrap();
        assert_eq!(lower.from(), Some(&2020));
        assert_eq!(lower.to(), None);

        let upper = TestRange::try_new(None, Some(2020)).unwrap();
        assert_eq!(upper.from(), None);
        assert_eq!(upper.to(), Some(&2020));
    }

    #[test]
    fn an_ascending_range_is_valid() {
        let res = TestRange::try_new(Some(2010), Some(2015));
        assert!(res.is_ok());
    }

    #[test]
    fn a_degenerate_range_is_not_inverted() {
        let res = TestRange::try_new(Some(2010), Some(2010));
        assert!(
            res.is_ok(),
            "one year is a valid span and an empty half-open interval"
        );
    }

    #[test]
    fn an_inverted_range_is_rejected() {
        let res = TestRange::try_new(Some(2015), Some(2010));
        assert!(res.is_err());
    }

    #[test]
    fn an_inverted_range_error_names_the_range() {
        let err = TestRange::try_new(Some(2015), Some(2010))
            .unwrap_err()
            .to_string();

        assert!(err.contains("test range"));
    }
}
