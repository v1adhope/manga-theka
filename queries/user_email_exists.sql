select exists (
    select 1
    from users
    where email = $1
) as "exists!";
