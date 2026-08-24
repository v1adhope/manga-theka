select r.id, r.chapter_id, r.version,
       l.id as language_id, l.code as language_code, l.name as language_name
from chapter_releases r
join languages l on l.id = r.language_id
where r.id = $1;
