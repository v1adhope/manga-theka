update book_covers
set is_main = false
where book_id = $1 and is_main;
