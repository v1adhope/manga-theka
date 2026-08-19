delete from chapters
where id = $1 and book_id = $2
returning id;
