select id
from chapter_pages
where release_id = $1 and sort_order = $2;
