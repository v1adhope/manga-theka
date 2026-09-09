select id, password_hash, roles
from users
where email = $1;
