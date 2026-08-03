select id, first_name, last_name, role, created_at
from creators
where id = $1;
