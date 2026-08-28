select exists(
    select 1
    from book_covers
    where id = $1
) as "exists!";
