select u.id, u.email, u.username, u.password_hash, u.roles, u.verified_at, u.created_at
from users u
where u.id = $1;
