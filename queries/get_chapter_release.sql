select r.id, r.chapter_id, r.version, r.created_by,
       l.id as language_id, l.code as language_code, l.name as language_name,
       (select count(*)
        from chapter_pages p
        where p.release_id = r.id and p.sort_order is not null) as "page_count!"
from chapter_releases r
join languages l on l.id = r.language_id
where r.id = $1;
