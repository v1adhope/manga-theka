select id, first_name, last_name, created_at
from creators
where id = $1;
