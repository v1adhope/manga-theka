select count(*) as "count!"
from chapter_pages
where release_id = $1;
