select
    exists (
        select 1
        from users
        where email = $1
    ) as "email_taken!",
    exists (
        select 1
        from users
        where username = $2
    ) as "username_taken!";
