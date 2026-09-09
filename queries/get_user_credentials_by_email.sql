select u.id, u.password_hash, u.roles
from users u
where u.email = $1;
