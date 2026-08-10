select id, kind, url
from book_links
where book_id = $1
order by id;
