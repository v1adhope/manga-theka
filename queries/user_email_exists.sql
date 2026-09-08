select exists (
    select 1
    from users u
    where u.email = $1
) as "exists!";
