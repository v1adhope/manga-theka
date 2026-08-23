delete from chapter_releases r
using chapters c
where c.id = r.chapter_id and c.book_id = $1;
