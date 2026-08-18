select id, book_id, extension, is_main
from book_covers
where book_id = $1
order by id;
