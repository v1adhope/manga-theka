update creators
set first_name = $2, last_name = $3, role = $4
where id = $1
returning id;