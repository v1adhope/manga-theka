select exists(
    select 1
    from chapter_pages
    where id = $1 and release_id = $2
) as "exists!";
