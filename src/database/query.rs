use crate::entity::{RangeBound, SortOrder};

pub(super) fn cursor_op(order: SortOrder) -> (&'static str, &'static str) {
    match order {
        SortOrder::Asc => (">", "asc"),
        SortOrder::Desc => ("<", "desc"),
    }
}

pub(super) fn fetch_limit(limit: i64) -> i64 {
    limit + 1
}

pub(super) fn take_page<R, C>(
    rows: &mut Vec<R>,
    limit: i64,
    cursor_of: impl Fn(&R) -> C,
) -> Option<C> {
    if rows.len() <= limit as usize {
        return None;
    }

    rows.pop();
    rows.last().map(cursor_of)
}

pub(super) fn upper_bound_op<B: RangeBound>() -> &'static str {
    if B::UPPER_INCLUSIVE { "<=" } else { "<" }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::{
        database::query::{cursor_op, fetch_limit, take_page, upper_bound_op},
        entity::{RangeBound, SortOrder},
    };

    struct InclusiveBound;

    impl RangeBound for InclusiveBound {
        const UPPER_INCLUSIVE: bool = true;
        const NAME: &'static str = "inclusive";
    }

    struct ExclusiveBound;

    impl RangeBound for ExclusiveBound {
        const UPPER_INCLUSIVE: bool = false;
        const NAME: &'static str = "exclusive";
    }

    struct Row {
        id: Uuid,
    }

    fn rows(n: u128) -> Vec<Row> {
        (1..=n)
            .map(|i| Row {
                id: Uuid::from_u128(i),
            })
            .collect()
    }

    #[test]
    fn ascending_order_compares_forward() {
        assert_eq!(cursor_op(SortOrder::Asc), (">", "asc"));
    }

    #[test]
    fn descending_order_compares_backward() {
        assert_eq!(cursor_op(SortOrder::Desc), ("<", "desc"));
    }

    #[test]
    fn an_upper_bound_reads_its_inclusivity_off_the_type() {
        assert_eq!(upper_bound_op::<InclusiveBound>(), "<=");
        assert_eq!(upper_bound_op::<ExclusiveBound>(), "<");
    }

    #[test]
    fn fetch_limit_asks_for_one_extra_row() {
        assert_eq!(fetch_limit(0), 1);
        assert_eq!(fetch_limit(1), 2);
        assert_eq!(fetch_limit(i64::from(u32::MAX)), i64::from(u32::MAX) + 1);
    }

    #[test]
    fn page_shorter_than_limit_has_no_next_cursor() {
        let mut rows = rows(2);

        let next_cursor = take_page(&mut rows, 3, |r| r.id);

        assert_eq!(next_cursor, None);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn full_page_without_an_extra_row_has_no_next_cursor() {
        let mut rows = rows(3);

        let next_cursor = take_page(&mut rows, 3, |r| r.id);

        assert_eq!(next_cursor, None);
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn extra_row_is_dropped_and_the_last_kept_row_is_the_next_cursor() {
        let mut rows = rows(4);

        let next_cursor = take_page(&mut rows, 3, |r| r.id);

        assert_eq!(next_cursor, Some(Uuid::from_u128(3)));
        assert_eq!(rows.len(), 3);
        assert_eq!(rows.last().unwrap().id, Uuid::from_u128(3));
    }

    #[test]
    fn empty_page_has_no_next_cursor() {
        let mut rows = rows(1);

        let next_cursor = take_page(&mut rows, 0, |r| r.id);

        assert_eq!(next_cursor, None);
        assert!(rows.is_empty());
    }
}
