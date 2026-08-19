select exists(
    select 1
    from book_covers
    where id = $1 and book_id = $2
) as "exists!";
