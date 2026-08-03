delete from creators
where id = $1
returning id;