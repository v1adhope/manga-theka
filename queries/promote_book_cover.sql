update book_covers
set is_main = true
where id = $2 and book_id = $1
returning id;
