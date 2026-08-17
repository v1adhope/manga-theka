select extension
from book_covers
where id = $1 and book_id = $2;
