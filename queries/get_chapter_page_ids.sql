select p.id
from chapter_pages p
join chapter_releases r on r.id = p.release_id
where r.chapter_id = $1;
