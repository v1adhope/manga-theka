select exists(
    select 1
    from chapter_releases
    where id = $1
) as "exists!";
