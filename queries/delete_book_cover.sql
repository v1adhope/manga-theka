delete from book_covers
where id = $1
returning id;
