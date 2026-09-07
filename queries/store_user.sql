insert into users(id, email, username, password_hash, roles, verified_at, created_at)
values($1, $2, $3, $4, $5, $6, $7);
