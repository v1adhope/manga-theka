select exists(
    select 1
    from chapters
    where id = $1
) as "exists!";
