delete from chapters
where id = $1
returning id;
