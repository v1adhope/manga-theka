select book_id, kind, url
from book_links
where book_id = any($1::uuid[])
order by book_id, kind, url;
