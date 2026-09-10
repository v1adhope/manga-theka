select id, email, username, roles, verified_at, created_at
from users
where id = $1;
