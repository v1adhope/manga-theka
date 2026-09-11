select id, release_id, sort_order as "sort_order!", extension
from chapter_pages
where release_id = $1 and sort_order is not null
order by sort_order;
