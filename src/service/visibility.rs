use crate::{
    entity::{BookVisibility, Entity, UserClaims},
    error::{DatabaseError, EntityError, ServiceError},
};

pub(super) fn ensure_book_writable(visibility: BookVisibility) -> Result<(), ServiceError> {
    match visibility {
        BookVisibility::Draft | BookVisibility::Listed => Ok(()),
        blocked => Err(EntityError::BookNotWritable(blocked).into()),
    }
}

pub(super) fn ensure_content_writable(visibility: BookVisibility) -> Result<(), ServiceError> {
    match visibility {
        BookVisibility::Listed => Ok(()),
        blocked => Err(EntityError::BookNotWritable(blocked).into()),
    }
}

// `T` is the resource the caller asked for, not the book: refusing to serve unlisted content
// must be indistinguishable from that resource never having existed, or the 404 becomes an
// oracle telling anyone holding an id that a book was moderated away rather than deleted.
pub(super) fn ensure_readable<T: Entity>(
    visibility: BookVisibility,
    claims: Option<&UserClaims>,
) -> Result<(), ServiceError> {
    // deferred: also admit the book's submitter once `books` records one (issue #3)
    if visibility == BookVisibility::Listed || claims.is_some_and(UserClaims::can_moderate) {
        return Ok(());
    }

    Err(DatabaseError::not_found::<T>().into())
}
