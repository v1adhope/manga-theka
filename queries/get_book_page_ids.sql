select p.id
from chapter_pages p
join chapter_releases r on r.id = p.release_id
join chapters c on c.id = r.chapter_id
where c.book_id = $1;
