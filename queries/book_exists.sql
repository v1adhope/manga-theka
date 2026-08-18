select exists(
    select 1
    from books
    where id = $1
) as "exists!";
