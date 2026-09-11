delete from books
where id = $1
returning id;
